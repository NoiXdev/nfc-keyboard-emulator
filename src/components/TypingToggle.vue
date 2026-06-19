<script setup lang="ts">
defineProps<{ enabled: boolean }>();
defineEmits<{ toggle: [value: boolean] }>();
</script>

<template>
  <button class="toggle" :class="{ on: enabled }" @click="$emit('toggle', !enabled)">
    {{ enabled ? "Tippen AKTIV" : "Tippen aus" }}
  </button>
</template>

<style scoped>
.toggle { padding: 12px 20px; font-size: 16px; border-radius: 8px; border: 1px solid #8884; }
.toggle.on { background: #14b8a6; color: #04201c; border-color: #14b8a6; }
</style>
