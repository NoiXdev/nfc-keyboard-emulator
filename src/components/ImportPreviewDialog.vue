<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ImportPreview, TokenChange } from "../lib/tauri";

defineProps<{ preview: ImportPreview }>();
defineEmits<{ apply: [id: string]; cancel: [id: string] }>();

const { t } = useI18n();

const onOff = (on: boolean) => t(on ? "import.on" : "import.off");
const tokenText = (c: TokenChange) =>
  t({ unchanged: "import.tokenUnchanged", set: "import.tokenSet", cleared: "import.tokenCleared" }[c]);
</script>

<template>
  <div class="overlay">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="import-title">
      <h2 id="import-title">{{ t("import.title") }}</h2>
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
        <dd data-test="http-host">{{ preview.httpHost ?? t("import.noHost") }}</dd>
        <dt>{{ t("import.tokenHttp") }}</dt>
        <dd data-test="http-token">{{ tokenText(preview.httpToken) }}</dd>

        <dt>{{ t("import.wsStream") }}</dt>
        <dd data-test="ws-stream">{{ onOff(preview.wsStream) }}</dd>
        <dt>{{ t("import.wsRequest") }}</dt>
        <dd data-test="ws-request">{{ onOff(preview.wsRequest) }}</dd>
        <dt>{{ t("import.host") }}</dt>
        <dd data-test="ws-host">{{ preview.wsHost ?? t("import.noHost") }}</dd>
        <dt>{{ t("import.tokenWs") }}</dt>
        <dd data-test="ws-token">{{ tokenText(preview.wsToken) }}</dd>
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
