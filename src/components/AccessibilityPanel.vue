<script setup lang="ts">
import { useI18n } from "vue-i18n";
defineProps<{ ok: boolean }>();
defineEmits<{ fix: [] }>();
const { t } = useI18n();
</script>

<template>
  <section class="panel">
    <div class="status" :class="ok ? 'ok' : 'warn'">
      <span class="dot"></span>
      <span>{{ ok ? t("a11y.ok") : t("a11y.missing") }}</span>
    </div>

    <p class="muted">{{ t("a11y.intro") }}</p>

    <ol class="steps">
      <li>{{ t("a11y.step1") }}</li>
      <li>{{ t("a11y.step2") }}</li>
      <li>{{ t("a11y.step3") }}</li>
    </ol>

    <div class="actions">
      <button class="primary" @click="$emit('fix')">{{ t("a11y.button") }}</button>
    </div>
  </section>
</template>

<style scoped>
.panel {
  display: grid;
  gap: 14px;
  max-width: 560px;
}
.status {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 14px;
}
.status.ok {
  background: #e7f8f4;
  color: #0f5f53;
}
.status.warn {
  background: #fdeceb;
  color: #a23b32;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status.ok .dot {
  background: #14b8a6;
}
.status.warn .dot {
  background: #f97066;
}
.muted {
  margin: 0;
  color: #5b6478;
  font-size: 14px;
  line-height: 1.6;
}
.steps {
  margin: 0;
  padding-left: 20px;
  color: #1b2030;
  font-size: 14px;
  line-height: 1.9;
}
.actions {
  margin-top: 4px;
}
.primary {
  padding: 9px 16px;
  border: none;
  border-radius: 8px;
  background: #14b8a6;
  color: #04201c;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}
.primary:hover {
  background: #10a092;
}
</style>
