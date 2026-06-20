<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ScanRecord } from "../lib/tauri";
defineProps<{ scans: ScanRecord[] }>();
defineEmits<{ export: [] }>();
const { t } = useI18n();
</script>

<template>
  <div class="wrap">
    <div class="toolbar">
      <span class="count">{{ scans.length }} {{ t("log.scans") }}</span>
      <button class="btn" data-test="export" @click="$emit('export')">{{ t("log.export") }}</button>
    </div>

    <div class="card">
      <table>
        <thead>
          <tr>
            <th>{{ t("log.time") }}</th>
            <th>{{ t("log.reader") }}</th>
            <th>{{ t("log.uid") }}</th>
            <th>{{ t("log.length") }}</th>
            <th>{{ t("log.typed") }}</th>
            <th>{{ t("log.status") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, i) in scans" :key="i">
            <td>{{ new Date(s.timestamp).toLocaleTimeString() }}</td>
            <td class="ellipsis">{{ s.reader }}</td>
            <td class="mono">{{ s.uidHex }}</td>
            <td>{{ s.uidLength }}</td>
            <td>{{ s.typed ? "✓" : "—" }}</td>
            <td><span class="status" :class="s.status">{{ t(`status.${s.status}`) }}</span></td>
          </tr>
        </tbody>
      </table>
      <p v-if="!scans.length" class="empty">{{ t("log.empty") }}</p>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  display: grid;
  gap: 12px;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.count {
  font-size: 13px;
  color: #5b6478;
}
.btn {
  padding: 8px 13px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
}
.btn:hover {
  background: #e9ecf6;
}
.card {
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  overflow: hidden;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}
th {
  text-align: left;
  padding: 10px 12px;
  background: #f4f5fa;
  color: #5b6478;
  font-weight: 600;
  border-bottom: 1px solid #e4e7f0;
}
td {
  padding: 9px 12px;
  border-bottom: 1px solid #eef0f6;
}
tbody tr:last-child td {
  border-bottom: none;
}
.mono {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  color: #1e2a6b;
}
.ellipsis {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.status {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 999px;
  background: #eef0f6;
  color: #5b6478;
}
.status.ok {
  background: #e7f8f4;
  color: #0f5f53;
}
.status.read_error,
.status.type_error,
.status.no_accessibility {
  background: #fdeceb;
  color: #a23b32;
}
.empty {
  margin: 0;
  padding: 20px 12px;
  text-align: center;
  font-size: 13px;
  color: #8b93a7;
}
</style>
