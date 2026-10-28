<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  api,
  onScan,
  onReadersChanged,
  onReaderStatus,
  onDelivery,
  onWsStatus,
  onWsRequest,
  onImportPreview,
  onImportError,
  type Config,
  type ConfigError,
  type DeliveryResult,
  type FormatConfig,
  type ImportError,
  type ImportPreview,
  type IntegrationConfig,
  type ReaderStatus,
  type ScanRecord,
  type WsStatus,
} from "./lib/tauri";
import { pushScan } from "./lib/scanLog";
import {
  applyDelivery,
  deliveryErrorText,
  serverErrorText,
  withPendingDelivery,
  wsNotice,
  type LoggedScan,
} from "./lib/delivery";
import { type NavView } from "./lib/nav";
import { setLocale, type Locale } from "./i18n";
import { checkForUpdate, type UpdateInfo } from "./lib/update";
import NavSidebar from "./components/NavSidebar.vue";
import ReaderSelect from "./components/ReaderSelect.vue";
import TypingToggle from "./components/TypingToggle.vue";
import FormatSettings from "./components/FormatSettings.vue";
import ScanLog from "./components/ScanLog.vue";
import StatusBar from "./components/StatusBar.vue";
import ServerSettings from "./components/ServerSettings.vue";
import ImportPreviewDialog from "./components/ImportPreviewDialog.vue";
import AccessibilityPanel from "./components/AccessibilityPanel.vue";
import AboutPanel from "./components/AboutPanel.vue";
import Toggle from "./components/Toggle.vue";
import Select from "./components/Select.vue";
import {
  enable as enableAutostart,
  disable as disableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

const { t, locale } = useI18n();

const config = ref<Config | null>(null);
const readers = ref<string[]>([]);
const scans = ref<LoggedScan[]>([]);
const status = ref<ReaderStatus | null>(null);
const ws = ref<WsStatus | null>(null);
const saveError = ref<ConfigError | null>(null);
const pendingImport = ref<ImportPreview | null>(null);
const importError = ref<ImportError | null>(null);
const typing = ref(false);
const accessibilityOk = ref(true);
const activeView = ref<NavView>("scanner");
const unlisten: Array<() => void> = [];

const autostart = ref(false);
const startMinimized = ref(false);
const update = ref<UpdateInfo | null>(null);
const checking = ref(false);

async function checkUpdates() {
  checking.value = true;
  update.value = await checkForUpdate();
  checking.value = false;
}

const languageOptions = [
  { value: "de", label: "Deutsch" },
  { value: "en", label: "English" },
];

type NotifyMode = "off" | "errors" | "all";
const notifyMode = ref<NotifyMode>(
  (localStorage.getItem("notify") as NotifyMode | null) ?? "all",
);
let notifyAllowed = false;
const notifyOptions = computed(() => [
  { value: "off", label: t("notify.off") },
  { value: "errors", label: t("notify.errors") },
  { value: "all", label: t("notify.all") },
]);
const ERROR_STATUS: ScanRecord["status"][] = [
  "read_error",
  "no_accessibility",
  "type_error",
];

async function ensureNotifyPermission() {
  notifyAllowed = await isPermissionGranted();
  if (!notifyAllowed) notifyAllowed = (await requestPermission()) === "granted";
}
async function setNotifyMode(value: string) {
  notifyMode.value = value as NotifyMode;
  localStorage.setItem("notify", value);
  if (value !== "off") await ensureNotifyPermission();
}
function notifyForScan(r: ScanRecord) {
  if (notifyMode.value === "off" || !notifyAllowed) return;
  const isError = ERROR_STATUS.includes(r.status);
  if (notifyMode.value === "errors" && !isError) return;
  let title: string;
  let body: string;
  switch (r.status) {
    case "ok":
      title = t("notify.scanned");
      body = t("notify.typed", { uid: r.uidHex });
      break;
    case "typing_disabled":
      title = t("notify.scanned");
      body = t("notify.notTyped", { uid: r.uidHex });
      break;
    case "read_error":
      title = t("status.read_error");
      body = t("notify.readError");
      break;
    case "no_accessibility":
      title = t("status.no_accessibility");
      body = t("notify.noAccessibility");
      break;
    case "type_error":
      title = t("status.type_error");
      body = t("notify.typeError", { uid: r.uidHex });
      break;
    default:
      return;
  }
  void sendNotification({ title, body });
}

// Delivery and connection problems count as errors: shown unless notifications are off.
// Text carries the target and the translated reason only, never a URL path or token.
function notifyServer(title: string, body: string) {
  if (notifyMode.value === "off" || !notifyAllowed) return;
  void sendNotification({ title, body });
}
function notifyDelivery(r: DeliveryResult) {
  if (r.ok) return;
  const target = t(r.target === "http" ? "notify.targetHttp" : "notify.targetWs");
  const error = deliveryErrorText(t, r.error ?? { kind: "unreachable", message: "" });
  notifyServer(t("notify.deliveryFailed"), t("notify.serverBody", { target, error }));
}
function notifyWsStatus(prev: WsStatus | null, next: WsStatus) {
  const notice = wsNotice(prev, next);
  if (notice?.kind === "reconnecting") {
    notifyServer(
      t("notify.serverConnectionLost"),
      t("notify.serverBody", {
        target: t("notify.targetWs"),
        error: deliveryErrorText(t, notice.error),
      }),
    );
  } else if (notice?.kind === "token_rejected") {
    notifyServer(t("notify.serverTokenRejected"), t("notify.serverTokenRejectedBody"));
  }
}

onMounted(async () => {
  config.value = await api.getConfig();
  typing.value = config.value.typingEnabledOnStart;
  readers.value = await api.listReaders();
  accessibilityOk.value = await api.checkAccessibility();
  unlisten.push(
    await onScan((r) => {
      scans.value = pushScan(scans.value, withPendingDelivery(r), config.value!.logRetention);
      notifyForScan(r);
    }),
  );
  unlisten.push(await onReadersChanged((r) => (readers.value = r)));
  unlisten.push(await onReaderStatus((s) => (status.value = s)));
  unlisten.push(
    await onDelivery((r) => {
      scans.value = applyDelivery(scans.value, r);
      notifyDelivery(r);
    }),
  );
  unlisten.push(
    await onWsStatus((s) => {
      notifyWsStatus(ws.value, s);
      ws.value = s;
    }),
  );
  unlisten.push(
    await onWsRequest(() => notifyServer(t("notify.serverRequest"), t("notify.serverWaiting"))),
  );
  // The same preview can arrive twice (replay after subscribing and the live event).
  unlisten.push(await onImportPreview((p) => (pendingImport.value = p)));
  unlisten.push(await onImportError((e) => (importError.value = e)));
  // An event that arrived while the snapshot was in flight is newer than the snapshot.
  const snapshot = await api.getWsStatus();
  ws.value ??= snapshot;
  autostart.value = await isAutostartEnabled();
  startMinimized.value = config.value!.startMinimized;
  if (notifyMode.value !== "off") await ensureNotifyPermission();
  void checkUpdates();
});
onUnmounted(() => unlisten.forEach((fn) => fn()));

function selectReader(name: string | null) {
  config.value!.selectedReader = name;
  api.selectReader(name);
}
function toggleTyping(value: boolean) {
  typing.value = value;
  api.setTypingEnabled(value);
}
function changeFormat(format: FormatConfig) {
  config.value!.format = format;
  api.updateFormat(format);
}
async function fixAccessibility() {
  await api.openAccessibilitySettings();
  accessibilityOk.value = await api.checkAccessibility();
}
async function exportCsv() {
  await api.exportLogCsv();
}
async function toggleAutostart(v: boolean) {
  if (v) await enableAutostart();
  else await disableAutostart();
  autostart.value = v;
}
function toggleStartMinimized(v: boolean) {
  startMinimized.value = v;
  api.setStartMinimized(v);
}
async function saveIntegration(integration: IntegrationConfig) {
  saveError.value = null;
  try {
    config.value!.integration = (await api.updateIntegration(integration)).integration;
  } catch (e) {
    saveError.value = e as ConfigError;
  }
}
async function exportServerConfig() {
  await api.exportServerConfig();
}
async function importServerConfig() {
  importError.value = null;
  try {
    const preview = await api.importServerConfig();
    if (preview) pendingImport.value = preview;
  } catch (e) {
    importError.value = e as ImportError;
  }
}
async function applyImport(id: string) {
  pendingImport.value = null;
  try {
    config.value = await api.applyImport(id);
  } catch (e) {
    importError.value = e as ImportError;
  }
}
function cancelImport(id: string) {
  pendingImport.value = null;
  void api.discardImport(id).catch(() => {});
}
function changeLanguage(v: string) {
  setLocale(v as Locale);
}
</script>

<template>
  <div class="app" v-if="config">
    <NavSidebar
      :active="activeView"
      :accessibility-ok="accessibilityOk"
      :update-available="update?.available ?? false"
      @navigate="activeView = $event"
    />

    <div class="content">
      <header class="head">
        <h1>{{ t(`view.${activeView}Title`) }}</h1>
        <p>{{ t(`view.${activeView}Sub`) }}</p>
      </header>

      <div class="body">
        <template v-if="activeView === 'scanner'">
          <ReaderSelect
            :readers="readers"
            :selected="config.selectedReader"
            @select="selectReader"
            @rescan="api.rescanReaders()"
          />
          <TypingToggle :enabled="typing" @toggle="toggleTyping" />
        </template>

        <ScanLog v-else-if="activeView === 'log'" :scans="scans" @export="exportCsv" />

        <template v-else-if="activeView === 'options'">
          <FormatSettings :format="config.format" @change="changeFormat" />

          <div class="card opt-card">
            <span class="opt-title">{{ t("options.startup") }}</span>
            <div class="opt-row">
              <Toggle :model-value="autostart" @update:model-value="toggleAutostart" />
              <span>{{ t("options.autostart") }}</span>
            </div>
            <div class="opt-row">
              <Toggle :model-value="startMinimized" @update:model-value="toggleStartMinimized" />
              <span>{{ t("options.startMinimized") }}</span>
            </div>
          </div>

          <div class="card opt-card">
            <span class="opt-title">{{ t("notify.label") }}</span>
            <div class="lang">
              <Select
                :model-value="notifyMode"
                :options="notifyOptions"
                @update:model-value="setNotifyMode"
              />
            </div>
          </div>

          <div class="card opt-card">
            <span class="opt-title">{{ t("options.language") }}</span>
            <div class="lang">
              <Select
                :model-value="locale"
                :options="languageOptions"
                @update:model-value="changeLanguage"
              />
            </div>
          </div>
        </template>

        <ServerSettings
          v-else-if="activeView === 'server'"
          :integration="config.integration"
          :ws="ws"
          :save-error="saveError"
          @save="saveIntegration"
          @export="exportServerConfig"
          @import="importServerConfig"
          @reconnect="api.wsReconnect()"
        />

        <AccessibilityPanel
          v-else-if="activeView === 'accessibility'"
          :ok="accessibilityOk"
          @fix="fixAccessibility"
        />

        <AboutPanel
          v-else
          :update="update"
          :checking="checking"
          @recheck="checkUpdates"
        />
      </div>

      <StatusBar :status="status" :typing="typing" :ws="ws" />
    </div>

    <ImportPreviewDialog
      v-if="pendingImport"
      :key="pendingImport.id"
      :preview="pendingImport"
      @apply="applyImport"
      @cancel="cancelImport"
    />

    <div v-if="importError" class="notice-overlay">
      <div class="notice" role="alertdialog" aria-modal="true" aria-labelledby="import-error-title">
        <h2 id="import-error-title">{{ t("import.errorTitle") }}</h2>
        <p>{{ serverErrorText(t, importError) }}</p>
        <button type="button" class="dismiss" @click="importError = null">
          {{ t("import.dismiss") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style>
:root {
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  color: #1b2030;
}
* {
  box-sizing: border-box;
}
body {
  margin: 0;
}
.app {
  display: flex;
  height: 100vh;
  background: #f4f5fa;
}
.content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: #f4f5fa;
}
.head {
  padding: 20px 24px 8px;
}
.head h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}
.head p {
  margin: 4px 0 0;
  font-size: 13px;
  color: #5b6478;
}
.body {
  flex: 1;
  overflow: auto;
  padding: 12px 24px 24px;
  display: grid;
  gap: 18px;
  align-content: start;
}
.opt-card {
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  padding: 16px;
  display: grid;
  gap: 12px;
}
.opt-title {
  font-size: 13px;
  font-weight: 600;
  color: #5b6478;
}
.opt-row {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
}
.lang {
  max-width: 220px;
}
.notice-overlay {
  position: fixed;
  inset: 0;
  background: rgba(27, 32, 48, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  z-index: 10;
}
.notice {
  background: #fff;
  border-radius: 12px;
  padding: 20px;
  width: min(420px, 100%);
  display: grid;
  gap: 14px;
}
.notice h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
}
.notice p {
  margin: 0;
  font-size: 14px;
}
.dismiss {
  justify-self: end;
  padding: 8px 14px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
}
</style>
