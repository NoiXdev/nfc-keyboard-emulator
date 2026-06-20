<script setup lang="ts">
defineProps<{ modelValue: boolean }>();
defineEmits<{ "update:modelValue": [value: boolean] }>();
</script>

<template>
  <button
    type="button"
    class="toggle"
    :class="{ on: modelValue }"
    role="switch"
    :aria-checked="modelValue"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span class="knob"></span>
  </button>
</template>

<style scoped>
.toggle {
  width: 40px;
  height: 23px;
  flex-shrink: 0;
  border: none;
  border-radius: 999px;
  background: #c2c8d6;
  padding: 0;
  cursor: pointer;
  position: relative;
  transition: background 0.15s ease;
}
.toggle.on {
  background: #14b8a6;
}
.knob {
  position: absolute;
  top: 2.5px;
  left: 2.5px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
  transition: transform 0.15s ease;
}
.toggle.on .knob {
  transform: translateX(17px);
}
</style>
