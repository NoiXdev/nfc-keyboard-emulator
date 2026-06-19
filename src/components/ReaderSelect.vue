<script setup lang="ts">
defineProps<{ readers: string[]; selected: string | null }>();
defineEmits<{ select: [name: string | null]; rescan: [] }>();
</script>

<template>
  <div class="row">
    <label>Scanner</label>
    <select :value="selected ?? ''" @change="$emit('select', ($event.target as HTMLSelectElement).value || null)">
      <option value="">— kein —</option>
      <option v-for="r in readers" :key="r" :value="r">{{ r }}</option>
    </select>
    <button @click="$emit('rescan')">Aktualisieren</button>
  </div>
</template>

<style scoped>
.row { display: flex; gap: 8px; align-items: center; }
</style>
