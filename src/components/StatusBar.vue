<script setup lang="ts">
import type { ReaderStatus } from "../lib/tauri";
defineProps<{ status: ReaderStatus | null; typing: boolean }>();
</script>

<template>
  <footer class="bar">
    <span class="item">
      <span class="dot" :class="status?.kind === 'connected' ? 'on' : 'off'"></span>
      Leser:
      {{
        status?.kind === "connected"
          ? "verbunden"
          : status?.kind === "error"
            ? `Fehler: ${status.message}`
            : "getrennt"
      }}
    </span>
    <span class="item">
      <span class="dot" :class="typing ? 'typing' : 'off'"></span>
      Tippen: {{ typing ? "aktiv" : "aus" }}
    </span>
  </footer>
</template>

<style scoped>
.bar {
  display: flex;
  gap: 18px;
  padding: 8px 18px;
  border-top: 1px solid #e4e7f0;
  font-size: 13px;
  color: #5b6478;
  background: #fff;
}
.item {
  display: flex;
  align-items: center;
  gap: 7px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.dot.on {
  background: #14b8a6;
}
.dot.typing {
  background: #f0b429;
}
.dot.off {
  background: #c2c8d6;
}
</style>
