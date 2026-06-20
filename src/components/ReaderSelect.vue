<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import Select from "./Select.vue";

const props = defineProps<{ readers: string[]; selected: string | null }>();
const emit = defineEmits<{ select: [name: string | null]; rescan: [] }>();

const { t } = useI18n();

const options = computed(() => [
  { value: "", label: t("reader.none") },
  ...props.readers.map((r) => ({ value: r, label: r })),
]);
</script>

<template>
  <div class="card">
    <label class="lbl">{{ t("reader.active") }}</label>
    <div class="row">
      <div class="grow">
        <Select
          :model-value="selected ?? ''"
          :options="options"
          @update:model-value="emit('select', $event || null)"
        />
      </div>
      <button class="btn" @click="emit('rescan')">{{ t("reader.refresh") }}</button>
    </div>
    <p v-if="!readers.length" class="empty">{{ t("reader.empty") }}</p>
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
.grow {
  flex: 1;
  min-width: 0;
}
.btn {
  padding: 9px 14px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
  white-space: nowrap;
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
