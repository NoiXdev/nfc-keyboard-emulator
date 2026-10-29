<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { AuthConfig, AuthMode, PayloadConfig, PayloadMode } from "../lib/tauri";
import { templateErrorText } from "../lib/delivery";
import {
  PLACEHOLDERS,
  defaultTemplate,
  examplePayload,
  renderTemplate,
} from "../lib/template";

const props = defineProps<{ target: "http" | "ws"; device: string }>();
const auth = defineModel<AuthConfig>("auth", { required: true });
const payload = defineModel<PayloadConfig>("payload", { required: true });

const { t } = useI18n();

const AUTH_MODES: AuthMode[] = ["bearer", "header", "basic", "query", "none"];
const PAYLOAD_MODES: PayloadMode[] = ["standard", "template"];

const id = (name: string) => `${props.target}-${name}`;

// Open on mount when something is customised; later toggling is left to the user.
const startOpen = auth.value.mode !== "bearer" || payload.value.mode !== "standard";

const summary = computed(() => {
  const parts: string[] = [];
  if (auth.value.mode === "none") parts.push(t("server.advanced.summaryNone"));
  else if (auth.value.mode !== "bearer") parts.push(t(`server.advanced.auth.${auth.value.mode}`));
  if (payload.value.mode === "template") parts.push(t("server.advanced.payload.template"));
  return parts.join(" · ");
});

function setAuthMode(mode: string) {
  auth.value = { ...auth.value, mode: mode as AuthMode };
}
function setAuthField(field: "headerName" | "username" | "queryParam", value: string) {
  auth.value = { ...auth.value, [field]: value };
}

function setPayloadMode(mode: string) {
  const next = mode as PayloadMode;
  const template =
    next === "template" && payload.value.template.trim() === ""
      ? defaultTemplate()
      : payload.value.template;
  payload.value = { mode: next, template };
}
function setTemplate(template: string) {
  payload.value = { ...payload.value, template };
}

const textarea = ref<HTMLTextAreaElement | null>(null);
const token = (name: string) => `{{${name}}}`;

async function insertPlaceholder(name: string) {
  const el = textarea.value;
  const text = payload.value.template;
  const start = el?.selectionStart ?? text.length;
  const end = el?.selectionEnd ?? text.length;
  const inserted = token(name);
  setTemplate(text.slice(0, start) + inserted + text.slice(end));
  await nextTick();
  el?.focus();
  el?.setSelectionRange(start + inserted.length, start + inserted.length);
}

const rendered = computed(() => renderTemplate(payload.value.template, examplePayload(props.device)));
const previewText = computed(() =>
  rendered.value.ok ? JSON.stringify(rendered.value.value, null, 2) : "",
);
const errorText = computed(() =>
  rendered.value.ok ? "" : templateErrorText(t, rendered.value.error),
);
</script>

<template>
  <details class="advanced" :open="startOpen" :data-test="id('advanced')">
    <summary>
      <span>{{ t("server.advanced.title") }}</span>
      <span v-if="summary" class="muted" :data-test="id('advanced-summary')">{{ summary }}</span>
    </summary>

    <div class="body">
      <div class="grid">
        <label :for="id('auth-mode')">{{ t("server.advanced.authMode") }}</label>
        <select
          :id="id('auth-mode')"
          :value="auth.mode"
          :data-test="id('auth-mode')"
          @change="setAuthMode(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="m in AUTH_MODES" :key="m" :value="m">
            {{ t(`server.advanced.auth.${m}`) }}
          </option>
        </select>

        <template v-if="auth.mode === 'header'">
          <label :for="id('header-name')">{{ t("server.advanced.headerName") }}</label>
          <input
            :id="id('header-name')"
            :value="auth.headerName"
            :data-test="id('header-name')"
            @input="setAuthField('headerName', ($event.target as HTMLInputElement).value)"
          />
        </template>
        <template v-else-if="auth.mode === 'basic'">
          <label :for="id('username')">{{ t("server.advanced.username") }}</label>
          <input
            :id="id('username')"
            :value="auth.username"
            :data-test="id('username')"
            autocomplete="off"
            @input="setAuthField('username', ($event.target as HTMLInputElement).value)"
          />
        </template>
        <template v-else-if="auth.mode === 'query'">
          <label :for="id('query-param')">{{ t("server.advanced.queryParam") }}</label>
          <input
            :id="id('query-param')"
            :value="auth.queryParam"
            :data-test="id('query-param')"
            @input="setAuthField('queryParam', ($event.target as HTMLInputElement).value)"
          />
        </template>
      </div>
      <div v-if="auth.mode === 'query'" class="warning" :data-test="id('query-warning')">
        {{ t("server.advanced.queryWarning") }}
      </div>

      <div class="grid">
        <label :for="id('payload-mode')">{{ t("server.advanced.payloadMode") }}</label>
        <select
          :id="id('payload-mode')"
          :value="payload.mode"
          :data-test="id('payload-mode')"
          @change="setPayloadMode(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="m in PAYLOAD_MODES" :key="m" :value="m">
            {{ t(`server.advanced.payload.${m}`) }}
          </option>
        </select>
      </div>

      <template v-if="payload.mode === 'template'">
        <textarea
          ref="textarea"
          class="template"
          rows="10"
          spellcheck="false"
          :aria-label="t('server.advanced.template')"
          :value="payload.template"
          :data-test="id('template')"
          @input="setTemplate(($event.target as HTMLTextAreaElement).value)"
        ></textarea>
        <div class="placeholders">
          <span class="muted">{{ t("server.advanced.placeholders") }}</span>
          <button
            v-for="name in PLACEHOLDERS"
            :key="name"
            type="button"
            class="btn chip"
            :data-test="id(`placeholder-${name}`)"
            @click="insertPlaceholder(name)"
          >
            {{ token(name) }}
          </button>
        </div>
        <div class="preview">
          <span class="muted">{{ t("server.advanced.preview") }}</span>
          <div v-if="errorText" class="fail" role="alert" :data-test="id('template-error')">
            {{ errorText }}
          </div>
          <pre v-else :data-test="id('preview')">{{ previewText }}</pre>
        </div>
        <p class="muted note" :data-test="id('preview-note')">
          {{ t("server.advanced.previewNote") }}
        </p>
        <p class="muted note" :data-test="id('test-note')">{{ t("server.advanced.testNote") }}</p>
        <p v-if="target === 'ws'" class="muted note" :data-test="id('request-note')">
          {{ t("server.advanced.requestNote") }}
        </p>
      </template>
    </div>
  </details>
</template>

<style scoped>
.advanced {
  border-top: 1px solid #e4e7f0;
  padding-top: 12px;
}
summary {
  display: flex;
  align-items: baseline;
  gap: 10px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 600;
  color: #5b6478;
}
summary .muted {
  font-weight: 400;
  font-size: 13px;
}
.body {
  display: grid;
  gap: 12px;
  margin-top: 12px;
}
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
input,
select,
textarea {
  padding: 9px 11px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  font-size: 14px;
  background: #fff;
  min-width: 0;
}
.template {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 13px;
  resize: vertical;
}
.placeholders {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
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
.btn:hover {
  background: #e9ecf6;
}
.btn.chip {
  padding: 3px 8px;
  font-size: 12px;
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
}
.preview {
  display: grid;
  gap: 6px;
  font-size: 13px;
}
.preview pre {
  margin: 0;
  padding: 10px 12px;
  background: #f4f5fa;
  border-radius: 8px;
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 13px;
  color: #1e2a6b;
  overflow: auto;
}
.fail {
  color: #b42318;
}
.muted {
  color: #5b6478;
}
.note {
  margin: 0;
  font-size: 13px;
}
.warning {
  padding: 12px;
  border-radius: 8px;
  background: #fff7e0;
  border: 1px solid #f0b429;
  font-size: 13px;
}
</style>
