<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  api,
  type ConfigError,
  type DeliveryError,
  type IntegrationConfig,
  type WsStatus,
} from "../lib/tauri";
import { deliveryErrorText, serverErrorText } from "../lib/delivery";
import Toggle from "./Toggle.vue";

const props = defineProps<{
  integration: IntegrationConfig;
  ws: WsStatus | null;
  saveError?: ConfigError | null;
}>();
const emit = defineEmits<{
  save: [integration: IntegrationConfig];
  export: [];
  import: [];
  reconnect: [];
}>();

const { t } = useI18n();

const clone = (i: IntegrationConfig): IntegrationConfig => JSON.parse(JSON.stringify(i));
const local = reactive<IntegrationConfig>(clone(props.integration));

// The parent hands back the stored (normalized) values after a save or an import.
watch(
  () => props.integration,
  (i) => Object.assign(local, clone(i)),
);

const dirty = computed(() => JSON.stringify(local) !== JSON.stringify(props.integration));

function save() {
  emit("save", clone(local));
}

const showHttpToken = ref(false);
const showWsToken = ref(false);

// null = no result yet, "running" = test in flight, "ok" or the failure otherwise.
type TestResult = null | "running" | "ok" | DeliveryError;
const httpTest = ref<TestResult>(null);
const wsTest = ref<TestResult>(null);

watch(
  () => local.http,
  () => (httpTest.value = null),
  { deep: true },
);
watch(
  () => local.ws,
  () => (wsTest.value = null),
  { deep: true },
);

async function runTest(result: typeof httpTest, run: () => Promise<void>) {
  result.value = "running";
  try {
    await run();
    result.value = "ok";
  } catch (e) {
    result.value = e as DeliveryError;
  }
}
const testHttp = () => runTest(httpTest, () => api.testHttp({ ...local.http }, local.deviceName));
const testWs = () => runTest(wsTest, () => api.testWs({ ...local.ws }, local.deviceName));

function testText(r: TestResult): string {
  if (r === "ok") return t("server.testOk");
  if (r && r !== "running") return deliveryErrorText(t, r);
  return "";
}

const wsStateText = computed(() => {
  const ws = props.ws;
  if (!ws) return t("server.state.off");
  return ws.state === "reconnecting"
    ? t("server.state.reconnecting", { error: deliveryErrorText(t, ws.error) })
    : t(`server.state.${ws.state}`);
});
const canReconnect = computed(
  () => props.ws?.state === "token_rejected" || props.ws?.state === "reconnecting",
);

const confirmingExport = ref(false);
function confirmExport() {
  confirmingExport.value = false;
  emit("export");
}
</script>

<template>
  <div class="card opt-card">
    <label class="opt-title" for="server-device">{{ t("server.deviceName") }}</label>
    <input
      id="server-device"
      v-model="local.deviceName"
      data-test="device-name"
      :placeholder="t('server.deviceNamePlaceholder')"
    />
  </div>

  <div class="card opt-card">
    <span class="opt-title">{{ t("server.httpTitle") }}</span>
    <div class="opt-row">
      <Toggle v-model="local.http.enabled" data-test="http-enabled" />
      <span>{{ t("server.httpEnabled") }}</span>
    </div>
    <div class="grid">
      <label for="http-url">{{ t("server.url") }}</label>
      <input id="http-url" v-model="local.http.url" data-test="http-url" placeholder="https://" />
      <label for="http-token">{{ t("server.token") }}</label>
      <div class="token">
        <input
          id="http-token"
          v-model="local.http.token"
          data-test="http-token"
          :type="showHttpToken ? 'text' : 'password'"
          autocomplete="off"
        />
        <button
          type="button"
          class="btn"
          data-test="http-token-reveal"
          @click="showHttpToken = !showHttpToken"
        >
          {{ showHttpToken ? t("server.hideToken") : t("server.showToken") }}
        </button>
      </div>
    </div>
    <div class="actions">
      <button
        type="button"
        class="btn"
        data-test="http-test"
        :disabled="httpTest === 'running'"
        @click="testHttp"
      >
        {{ t("server.test") }}
      </button>
      <span
        v-if="httpTest && httpTest !== 'running'"
        class="result"
        :class="httpTest === 'ok' ? 'ok' : 'fail'"
        data-test="http-test-result"
        >{{ testText(httpTest) }}</span
      >
    </div>
  </div>

  <div class="card opt-card">
    <span class="opt-title">{{ t("server.wsTitle") }}</span>
    <div class="grid">
      <label for="ws-url">{{ t("server.url") }}</label>
      <input id="ws-url" v-model="local.ws.url" data-test="ws-url" placeholder="wss://" />
      <label for="ws-token">{{ t("server.token") }}</label>
      <div class="token">
        <input
          id="ws-token"
          v-model="local.ws.token"
          data-test="ws-token"
          :type="showWsToken ? 'text' : 'password'"
          autocomplete="off"
        />
        <button
          type="button"
          class="btn"
          data-test="ws-token-reveal"
          @click="showWsToken = !showWsToken"
        >
          {{ showWsToken ? t("server.hideToken") : t("server.showToken") }}
        </button>
      </div>
    </div>
    <div class="opt-row">
      <Toggle v-model="local.ws.stream" data-test="ws-stream" />
      <span>{{ t("server.wsStream") }}</span>
    </div>
    <div class="opt-row">
      <Toggle v-model="local.ws.request" data-test="ws-request" />
      <span>{{ t("server.wsRequest") }}</span>
    </div>
    <div class="opt-row" data-test="ws-state">
      <span class="muted">{{ t("server.wsState") }}:</span>
      <span>{{ wsStateText }}</span>
      <button
        v-if="canReconnect"
        type="button"
        class="btn"
        data-test="ws-reconnect"
        @click="emit('reconnect')"
      >
        {{ t("server.reconnect") }}
      </button>
    </div>
    <div class="actions">
      <button
        type="button"
        class="btn"
        data-test="ws-test"
        :disabled="wsTest === 'running'"
        @click="testWs"
      >
        {{ t("server.test") }}
      </button>
      <span
        v-if="wsTest && wsTest !== 'running'"
        class="result"
        :class="wsTest === 'ok' ? 'ok' : 'fail'"
        data-test="ws-test-result"
        >{{ testText(wsTest) }}</span
      >
    </div>
  </div>

  <div class="actions save-row">
    <button type="button" class="btn primary" data-test="save" :disabled="!dirty" @click="save">
      {{ t("server.save") }}
    </button>
    <span v-if="saveError" class="result fail" role="alert" data-test="save-error">{{
      serverErrorText(t, saveError)
    }}</span>
  </div>

  <div class="card opt-card">
    <span class="opt-title">{{ t("server.transferTitle") }}</span>
    <div class="actions">
      <button type="button" class="btn" data-test="import" @click="emit('import')">
        {{ t("server.import") }}
      </button>
      <button type="button" class="btn" data-test="export" @click="confirmingExport = true">
        {{ t("server.export") }}
      </button>
    </div>
    <div v-if="confirmingExport" class="warning" role="alert" data-test="export-warning">
      <p>{{ t("server.exportWarning") }}</p>
      <div class="actions">
        <button type="button" class="btn" data-test="export-confirm" @click="confirmExport">
          {{ t("server.exportConfirm") }}
        </button>
        <button type="button" class="btn" data-test="export-cancel" @click="confirmingExport = false">
          {{ t("server.cancel") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.grid {
  display: grid;
  grid-template-columns: 140px 1fr;
  gap: 10px 14px;
  align-items: center;
}
.grid label {
  font-size: 14px;
  color: #5b6478;
}
input {
  padding: 9px 11px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  font-size: 14px;
  background: #fff;
  min-width: 0;
}
.token {
  display: flex;
  gap: 8px;
}
.token input {
  flex: 1;
}
.actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.btn {
  padding: 8px 13px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
  white-space: nowrap;
}
.btn:hover:not(:disabled) {
  background: #e9ecf6;
}
.btn:disabled {
  opacity: 0.55;
  cursor: default;
}
.btn.primary {
  background: #1e2a6b;
  border-color: #1e2a6b;
  color: #fff;
}
.btn.primary:hover:not(:disabled) {
  background: #2a3a8c;
}
.result {
  font-size: 13px;
}
.result.ok {
  color: #0f8a7d;
}
.result.fail {
  color: #b42318;
}
.muted {
  color: #5b6478;
}
.warning {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: 8px;
  background: #fff7e0;
  border: 1px solid #f0b429;
  font-size: 13px;
}
.warning p {
  margin: 0;
}
</style>
