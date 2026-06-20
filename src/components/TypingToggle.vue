<script setup lang="ts">
defineProps<{ enabled: boolean }>();
defineEmits<{ toggle: [value: boolean] }>();
</script>

<template>
  <button class="toggle" :class="{ on: enabled }" @click="$emit('toggle', !enabled)">
    <span class="dot"></span>
    <span class="text">
      <span class="title">{{ enabled ? "Tippen aktiv" : "Tippen aus" }}</span>
      <span class="hint">{{
        enabled
          ? "Gescannte UID wird ins fokussierte Feld getippt."
          : "Klicken zum Scharfschalten."
      }}</span>
    </span>
  </button>
</template>

<style scoped>
.toggle {
  display: flex;
  align-items: center;
  gap: 14px;
  width: 100%;
  padding: 16px 18px;
  border-radius: 12px;
  border: 1px solid #d4d9e6;
  background: #fff;
  cursor: pointer;
  text-align: left;
}
.toggle:hover {
  border-color: #b9c0d4;
}
.toggle.on {
  background: #14b8a6;
  border-color: #14b8a6;
}
.dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #c2c8d6;
  flex-shrink: 0;
}
.toggle.on .dot {
  background: #04201c;
}
.text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.title {
  font-size: 16px;
  font-weight: 600;
  color: #1b2030;
}
.toggle.on .title {
  color: #04201c;
}
.hint {
  font-size: 12px;
  color: #5b6478;
}
.toggle.on .hint {
  color: #053a33;
}
</style>
