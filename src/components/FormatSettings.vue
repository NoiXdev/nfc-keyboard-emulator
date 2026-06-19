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
watch(local, () => { emit("change", { ...local }); refresh(); }, { deep: true });
refresh();
</script>

<template>
  <fieldset>
    <legend>Format</legend>
    <label>Groß/klein
      <select v-model="local.case"><option value="upper">GROSS</option><option value="lower">klein</option></select>
    </label>
    <label>Trennzeichen
      <select v-model="local.separator"><option value="">keins</option><option value=":">:</option><option value="-">-</option><option value=" ">Leer</option></select>
    </label>
    <label>Byte-Reihenfolge
      <select v-model="local.byteOrder"><option value="normal">normal</option><option value="reversed">umgekehrt</option></select>
    </label>
    <label>Präfix <input v-model="local.prefix" /></label>
    <label>Abschluss
      <select v-model="local.suffix"><option value="enter">Enter</option><option value="tab">Tab</option><option value="none">keins</option></select>
    </label>
    <p class="preview">Vorschau: <code>{{ preview.text }}</code></p>
  </fieldset>
</template>

<style scoped>
fieldset { display: grid; gap: 8px; }
.preview code { font-family: monospace; }
</style>
