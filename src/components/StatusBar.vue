<script setup lang="ts">
import type { ReaderStatus } from "../lib/tauri";
defineProps<{ status: ReaderStatus | null; accessibilityOk: boolean; typing: boolean }>();
defineEmits<{ fixAccessibility: [] }>();
</script>

<template>
  <footer class="bar">
    <span>Leser:
      {{ status?.kind === "connected" ? "verbunden"
         : status?.kind === "error" ? `Fehler: ${status.message}` : "getrennt" }}
    </span>
    <span>Tippen: {{ typing ? "aktiv" : "aus" }}</span>
    <span v-if="!accessibilityOk" class="warn">
      Bedienungshilfen fehlen
      <button @click="$emit('fixAccessibility')">Einstellungen öffnen</button>
    </span>
  </footer>
</template>

<style scoped>
.bar { display: flex; gap: 16px; padding: 8px; border-top: 1px solid #8884; font-size: 13px; }
.warn { color: #c0392b; }
</style>
