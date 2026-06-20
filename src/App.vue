<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import {
  api,
  onScan,
  onReadersChanged,
  onReaderStatus,
  type Config,
  type FormatConfig,
  type ReaderStatus,
  type ScanRecord,
} from "./lib/tauri";
import { pushScan } from "./lib/scanLog";
import { VIEW_TITLES, type NavView } from "./lib/nav";
import NavSidebar from "./components/NavSidebar.vue";
import ReaderSelect from "./components/ReaderSelect.vue";
import TypingToggle from "./components/TypingToggle.vue";
import FormatSettings from "./components/FormatSettings.vue";
import ScanLog from "./components/ScanLog.vue";
import StatusBar from "./components/StatusBar.vue";
import AccessibilityPanel from "./components/AccessibilityPanel.vue";
import {
  enable as enableAutostart,
  disable as disableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";

const config = ref<Config | null>(null);
const readers = ref<string[]>([]);
const scans = ref<ScanRecord[]>([]);
const status = ref<ReaderStatus | null>(null);
const typing = ref(false);
const accessibilityOk = ref(true);
const activeView = ref<NavView>("scanner");
const unlisten: Array<() => void> = [];

const autostart = ref(false);
const startMinimized = ref(false);

const SUBTITLES: Record<NavView, string> = {
  scanner: "Leser wählen und das Tippen scharfschalten.",
  log: "Alle gescannten UIDs, neueste zuerst.",
  options: "Ausgabeformat und Startverhalten.",
  accessibility: "Berechtigung zum Tippen in andere Apps.",
};

onMounted(async () => {
  config.value = await api.getConfig();
  typing.value = config.value.typingEnabledOnStart;
  readers.value = await api.listReaders();
  accessibilityOk.value = await api.checkAccessibility();
  unlisten.push(
    await onScan((r) => {
      scans.value = pushScan(scans.value, r, config.value!.logRetention);
    }),
  );
  unlisten.push(await onReadersChanged((r) => (readers.value = r)));
  unlisten.push(await onReaderStatus((s) => (status.value = s)));
  autostart.value = await isAutostartEnabled();
  startMinimized.value = config.value!.startMinimized;
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
</script>

<template>
  <div class="app" v-if="config">
    <NavSidebar
      :active="activeView"
      :accessibility-ok="accessibilityOk"
      @navigate="activeView = $event"
    />

    <div class="content">
      <header class="head">
        <h1>{{ VIEW_TITLES[activeView] }}</h1>
        <p>{{ SUBTITLES[activeView] }}</p>
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
          <fieldset class="options">
            <legend>Start</legend>
            <label>
              <input
                type="checkbox"
                :checked="autostart"
                @change="toggleAutostart(($event.target as HTMLInputElement).checked)"
              />
              Autostart beim Systemstart
            </label>
            <label>
              <input
                type="checkbox"
                :checked="startMinimized"
                @change="toggleStartMinimized(($event.target as HTMLInputElement).checked)"
              />
              Minimiert starten
            </label>
          </fieldset>
        </template>

        <AccessibilityPanel
          v-else
          :ok="accessibilityOk"
          @fix="fixAccessibility"
        />
      </div>

      <StatusBar :status="status" :typing="typing" />
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
.options {
  display: grid;
  gap: 10px;
  border: 1px solid #e4e7f0;
  border-radius: 10px;
  padding: 14px 16px;
  background: #fff;
}
.options legend {
  font-weight: 600;
  font-size: 13px;
  padding: 0 6px;
}
.options label {
  display: flex;
  gap: 9px;
  align-items: center;
  font-size: 14px;
}
</style>
