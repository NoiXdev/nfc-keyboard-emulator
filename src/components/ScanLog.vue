<script setup lang="ts">
import type { ScanRecord } from "../lib/tauri";
defineProps<{ scans: ScanRecord[] }>();
defineEmits<{ export: [] }>();
</script>

<template>
  <section>
    <header class="row">
      <h2>Scan-Log</h2>
      <button data-test="export" @click="$emit('export')">CSV exportieren</button>
    </header>
    <table>
      <thead>
        <tr><th>Zeit</th><th>Leser</th><th>UID</th><th>Länge</th><th>Getippt</th><th>Status</th></tr>
      </thead>
      <tbody>
        <tr v-for="(s, i) in scans" :key="i">
          <td>{{ new Date(s.timestamp).toLocaleTimeString() }}</td>
          <td>{{ s.reader }}</td>
          <td class="mono">{{ s.uidHex }}</td>
          <td>{{ s.uidLength }}</td>
          <td>{{ s.typed ? "✓" : "—" }}</td>
          <td>{{ s.status }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.row { display: flex; justify-content: space-between; align-items: center; }
.mono { font-family: monospace; }
table { width: 100%; border-collapse: collapse; }
td, th { text-align: left; padding: 4px 8px; border-bottom: 1px solid #8884; }
</style>
