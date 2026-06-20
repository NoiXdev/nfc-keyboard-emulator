<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";

interface Option {
  value: string;
  label: string;
}

const props = defineProps<{
  modelValue: string;
  options: Option[];
  placeholder?: string;
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const open = ref(false);
const root = ref<HTMLElement | null>(null);

const selectedLabel = computed(
  () =>
    props.options.find((o) => o.value === props.modelValue)?.label ??
    props.placeholder ??
    "",
);

function choose(value: string) {
  emit("update:modelValue", value);
  open.value = false;
}

function onDocMouseDown(e: MouseEvent) {
  if (open.value && root.value && !root.value.contains(e.target as Node)) {
    open.value = false;
  }
}

onMounted(() => document.addEventListener("mousedown", onDocMouseDown));
onUnmounted(() => document.removeEventListener("mousedown", onDocMouseDown));
</script>

<template>
  <div class="select" ref="root">
    <button type="button" class="trigger" :class="{ open }" @click="open = !open">
      <span class="value">{{ selectedLabel }}</span>
      <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M6 9l6 6 6-6" />
      </svg>
    </button>
    <ul v-if="open" class="menu" role="listbox">
      <li
        v-for="o in options"
        :key="o.value"
        class="option"
        :class="{ active: o.value === modelValue }"
        role="option"
        :aria-selected="o.value === modelValue"
        @click="choose(o.value)"
      >
        <span class="opt-label">{{ o.label }}</span>
        <svg v-if="o.value === modelValue" class="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M5 12l4 4 10-10" />
        </svg>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.select {
  position: relative;
  width: 100%;
}
.trigger {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 9px 11px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #fff;
  font-size: 14px;
  color: #1b2030;
  cursor: pointer;
  text-align: left;
}
.trigger:hover {
  border-color: #b9c0d4;
}
.trigger.open {
  border-color: #14b8a6;
  box-shadow: 0 0 0 3px rgba(20, 184, 166, 0.15);
}
.value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.chevron {
  width: 16px;
  height: 16px;
  color: #8b93a7;
  flex-shrink: 0;
  transition: transform 0.15s ease;
}
.trigger.open .chevron {
  transform: rotate(180deg);
}
.menu {
  position: absolute;
  top: calc(100% + 5px);
  left: 0;
  right: 0;
  z-index: 30;
  margin: 0;
  padding: 5px;
  list-style: none;
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 9px;
  box-shadow: 0 8px 24px rgba(20, 30, 80, 0.14);
  max-height: 220px;
  overflow: auto;
}
.option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 9px;
  border-radius: 6px;
  font-size: 14px;
  color: #1b2030;
  cursor: pointer;
}
.option:hover {
  background: #f4f5fa;
}
.option.active {
  color: #0f766e;
  font-weight: 500;
}
.opt-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.check {
  width: 16px;
  height: 16px;
  color: #14b8a6;
  flex-shrink: 0;
}
</style>
