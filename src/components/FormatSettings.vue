<script setup lang="ts">
import { reactive, watch } from "vue";
import type { FormatConfig } from "../lib/tauri";
import { api } from "../lib/tauri";

const props = defineProps<{ format: FormatConfig }>();
const emit = defineEmits<{ change: [format: FormatConfig] }>();

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
</script>

<template>
  <div class="card">
    <div class="grid">
      <label>Groß/klein</label>
      <select v-model="local.case">
        <option value="upper">GROSS</option>
        <option value="lower">klein</option>
      </select>

      <label>Trennzeichen</label>
      <select v-model="local.separator">
        <option value="">keins</option>
        <option value=":">:</option>
        <option value="-">-</option>
        <option value=" ">Leerzeichen</option>
      </select>

      <label>Byte-Reihenfolge</label>
      <select v-model="local.byteOrder">
        <option value="normal">normal</option>
        <option value="reversed">umgekehrt</option>
      </select>

      <label>Präfix</label>
      <input v-model="local.prefix" placeholder="(leer)" />

      <label>Abschluss</label>
      <select v-model="local.suffix">
        <option value="enter">Enter</option>
        <option value="tab">Tab</option>
        <option value="none">keins</option>
      </select>
    </div>

    <div class="preview">
      <span class="preview-lbl">Vorschau</span>
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
.grid select,
.grid input {
  padding: 8px 10px;
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
