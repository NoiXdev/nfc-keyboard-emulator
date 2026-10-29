<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { ImportPreview, TokenChange } from "../lib/tauri";

const props = defineProps<{ preview: ImportPreview }>();
defineEmits<{ apply: [id: string]; cancel: [id: string] }>();

const { t } = useI18n();

const onOff = (on: boolean) => t(on ? "import.on" : "import.off");
const tokenText = (c: TokenChange) =>
  t({ unchanged: "import.tokenUnchanged", set: "import.tokenSet", cleared: "import.tokenCleared" }[c]);
const hostText = (host: string | null) => host ?? t("import.noHost");

// A link can point the stored token at another server; say so before anything is applied.
const warnings = computed(() => {
  const p = props.preview;
  const targets = [
    {
      name: t("import.targetHttp"),
      host: p.httpHostAfter,
      toNewHost: p.httpTokenToNewHost,
      downgrade: p.httpDowngrade,
    },
    {
      name: t("import.targetWs"),
      host: p.wsHostAfter,
      toNewHost: p.wsTokenToNewHost,
      downgrade: p.wsDowngrade,
    },
  ];
  return targets.flatMap(({ name, host, toNewHost, downgrade }) => [
    ...(toNewHost ? [t("import.warnTokenToNewHost", { target: name, host })] : []),
    ...(downgrade ? [t("import.warnDowngrade", { target: name })] : []),
  ]);
});
</script>

<template>
  <div class="overlay">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="import-title">
      <h2 id="import-title">{{ t("import.title") }}</h2>
      <ul v-if="warnings.length" class="warnings" role="alert" data-test="warnings">
        <li v-for="w in warnings" :key="w">{{ w }}</li>
      </ul>
      <dl>
        <dt>{{ t("import.source") }}</dt>
        <dd data-test="source">
          {{ t(preview.source === "file" ? "import.sourceFile" : "import.sourceLink") }}
        </dd>
        <dt>{{ t("import.deviceName") }}</dt>
        <dd data-test="device">{{ preview.deviceName }}</dd>

        <dt>{{ t("import.http") }}</dt>
        <dd data-test="http">{{ onOff(preview.httpEnabled) }}</dd>
        <dt>{{ t("import.host") }}</dt>
        <dd
          data-test="http-host"
          :class="{ changed: preview.httpHostBefore !== preview.httpHostAfter }"
        >
          <template v-if="preview.httpHostBefore !== preview.httpHostAfter">
            <s class="before">{{ hostText(preview.httpHostBefore) }}</s> →
          </template>
          {{ hostText(preview.httpHostAfter) }}
        </dd>
        <dt>{{ t("import.tokenHttp") }}</dt>
        <dd data-test="http-token" :class="{ changed: preview.httpToken !== 'unchanged' }">
          {{ tokenText(preview.httpToken) }}
        </dd>
        <template v-if="preview.httpAuthChanged">
          <dt>{{ t("server.advanced.authMode") }}</dt>
          <dd data-test="http-auth" class="changed">
            {{ t(`server.advanced.auth.${preview.httpAuth}`) }}
            <span class="marker">({{ t("import.changed") }})</span>
          </dd>
        </template>
        <template v-if="preview.httpPayloadChanged">
          <dt>{{ t("server.advanced.payloadMode") }}</dt>
          <dd data-test="http-payload" class="changed">
            {{ t(`server.advanced.payload.${preview.httpPayload}`) }}
            <span class="marker">({{ t("import.changed") }})</span>
          </dd>
        </template>

        <dt>{{ t("import.wsStream") }}</dt>
        <dd data-test="ws-stream">{{ onOff(preview.wsStream) }}</dd>
        <dt>{{ t("import.wsRequest") }}</dt>
        <dd data-test="ws-request">{{ onOff(preview.wsRequest) }}</dd>
        <dt>{{ t("import.host") }}</dt>
        <dd data-test="ws-host" :class="{ changed: preview.wsHostBefore !== preview.wsHostAfter }">
          <template v-if="preview.wsHostBefore !== preview.wsHostAfter">
            <s class="before">{{ hostText(preview.wsHostBefore) }}</s> →
          </template>
          {{ hostText(preview.wsHostAfter) }}
        </dd>
        <dt>{{ t("import.tokenWs") }}</dt>
        <dd data-test="ws-token" :class="{ changed: preview.wsToken !== 'unchanged' }">
          {{ tokenText(preview.wsToken) }}
        </dd>
        <template v-if="preview.wsAuthChanged">
          <dt>{{ t("server.advanced.authMode") }}</dt>
          <dd data-test="ws-auth" class="changed">
            {{ t(`server.advanced.auth.${preview.wsAuth}`) }}
            <span class="marker">({{ t("import.changed") }})</span>
          </dd>
        </template>
        <template v-if="preview.wsPayloadChanged">
          <dt>{{ t("server.advanced.payloadMode") }}</dt>
          <dd data-test="ws-payload" class="changed">
            {{ t(`server.advanced.payload.${preview.wsPayload}`) }}
            <span class="marker">({{ t("import.changed") }})</span>
          </dd>
        </template>
      </dl>
      <div class="buttons">
        <button type="button" class="btn" data-test="cancel" @click="$emit('cancel', preview.id)">
          {{ t("import.cancel") }}
        </button>
        <button
          type="button"
          class="btn primary"
          data-test="apply"
          @click="$emit('apply', preview.id)"
        >
          {{ t("import.apply") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(27, 32, 48, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  z-index: 10;
}
.dialog {
  background: #fff;
  border-radius: 12px;
  padding: 20px;
  width: min(460px, 100%);
  max-height: 100%;
  overflow: auto;
  display: grid;
  gap: 14px;
}
h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
}
dl {
  margin: 0;
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 8px 16px;
  font-size: 14px;
}
dt {
  color: #5b6478;
}
dd {
  margin: 0;
  overflow-wrap: anywhere;
}
dd.changed {
  font-weight: 600;
}
.marker {
  color: #5b6478;
  font-weight: 400;
}
.before {
  color: #5b6478;
  font-weight: 400;
}
.warnings {
  margin: 0;
  padding: 10px 12px 10px 28px;
  border: 1px solid #e0a43a;
  border-radius: 8px;
  background: #fff6e0;
  color: #6b4a00;
  font-size: 14px;
  display: grid;
  gap: 6px;
}
.buttons {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.btn {
  padding: 8px 14px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
}
.btn:hover {
  background: #e9ecf6;
}
.btn.primary {
  background: #1e2a6b;
  border-color: #1e2a6b;
  color: #fff;
}
.btn.primary:hover {
  background: #2a3a8c;
}
</style>
