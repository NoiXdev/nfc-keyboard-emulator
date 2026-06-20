<script setup lang="ts">
defineProps<{ readers: string[]; selected: string | null }>();
defineEmits<{ select: [name: string | null]; rescan: [] }>();
</script>

<template>
  <div class="card">
    <label class="lbl">Aktiver Leser</label>
    <div class="row">
      <select
        class="select"
        :value="selected ?? ''"
        @change="$emit('select', ($event.target as HTMLSelectElement).value || null)"
      >
        <option value="">— kein Leser gewählt —</option>
        <option v-for="r in readers" :key="r" :value="r">{{ r }}</option>
      </select>
      <button class="btn" @click="$emit('rescan')">Aktualisieren</button>
    </div>
    <p v-if="!readers.length" class="empty">
      Kein PC/SC-Leser gefunden. Leser anstecken und „Aktualisieren".
    </p>
  </div>
</template>

<style scoped>
.card {
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  padding: 16px;
  display: grid;
  gap: 10px;
}
.lbl {
  font-size: 13px;
  font-weight: 600;
  color: #5b6478;
}
.row {
  display: flex;
  gap: 10px;
}
.select {
  flex: 1;
  min-width: 0;
  padding: 9px 11px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  font-size: 14px;
  background: #fff;
}
.btn {
  padding: 9px 14px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
}
.btn:hover {
  background: #e9ecf6;
}
.empty {
  margin: 0;
  font-size: 13px;
  color: #8b93a7;
}
</style>
