<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { FormatConfig } from "../lib/tauri";
import { api } from "../lib/tauri";
import Select from "./Select.vue";

const props = defineProps<{ format: FormatConfig }>();
const emit = defineEmits<{ change: [format: FormatConfig] }>();

const { t } = useI18n();

const local = reactive<FormatConfig>({ ...props.format });
const preview = reactive({ text: "" });

async function refresh() {
  preview.text = await api.formatPreview({ ...local });
}
watch(
  local,
  () => {
    emit("change", { ...local });
    refresh();
  },
  { deep: true },
);
refresh();

const caseOptions = computed(() => [
  { value: "upper", label: t("format.upper") },
  { value: "lower", label: t("format.lower") },
]);
const separatorOptions = computed(() => [
  { value: "", label: t("format.sepNone") },
  { value: ":", label: ":" },
  { value: "-", label: "-" },
  { value: " ", label: t("format.sepSpace") },
]);
const byteOrderOptions = computed(() => [
  { value: "normal", label: t("format.normal") },
  { value: "reversed", label: t("format.reversed") },
]);
const suffixOptions = computed(() => [
  { value: "enter", label: t("format.enter") },
  { value: "tab", label: t("format.tab") },
  { value: "none", label: t("format.none") },
]);
</script>

<template>
  <div class="card">
    <div class="grid">
      <label>{{ t("format.case") }}</label>
      <Select
        :model-value="local.case"
        :options="caseOptions"
        @update:model-value="local.case = $event as FormatConfig['case']"
      />

      <label>{{ t("format.separator") }}</label>
      <Select
        :model-value="local.separator"
        :options="separatorOptions"
        @update:model-value="local.separator = $event"
      />

      <label>{{ t("format.byteOrder") }}</label>
      <Select
        :model-value="local.byteOrder"
        :options="byteOrderOptions"
        @update:model-value="local.byteOrder = $event as FormatConfig['byteOrder']"
      />

      <label>{{ t("format.prefix") }}</label>
      <input v-model="local.prefix" :placeholder="t('format.prefixPlaceholder')" />

      <label>{{ t("format.trailing") }}</label>
      <Select
        :model-value="local.suffix"
        :options="suffixOptions"
        @update:model-value="local.suffix = $event as FormatConfig['suffix']"
      />
    </div>

    <div class="preview">
      <span class="preview-lbl">{{ t("format.preview") }}</span>
      <code>{{ preview.text }}</code>
    </div>
  </div>
</template>

<style scoped>
.card {
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  padding: 16px;
  display: grid;
  gap: 14px;
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
.grid input {
  padding: 9px 11px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  font-size: 14px;
  background: #fff;
}
.preview {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: #f4f5fa;
  border-radius: 8px;
}
.preview-lbl {
  font-size: 12px;
  font-weight: 600;
  color: #5b6478;
}
.preview code {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 14px;
  color: #1e2a6b;
}
</style>
