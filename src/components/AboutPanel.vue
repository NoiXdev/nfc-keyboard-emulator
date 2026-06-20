<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { rustLicenses, npmLicenses } from "../lib/licenses";
import type { UpdateInfo } from "../lib/update";

defineProps<{ update: UpdateInfo | null; checking: boolean }>();
defineEmits<{ recheck: [] }>();

const { t } = useI18n();
const version = ref("");

onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    version.value = "";
  }
});
</script>

<template>
  <section class="about">
    <div class="hero">
      <svg class="logo" viewBox="0 0 1024 1024" aria-hidden="true">
        <rect width="1024" height="1024" rx="220" fill="#F4F5FA" />
        <g transform="rotate(-14 512 512)">
          <rect x="300" y="360" width="380" height="260" rx="40" fill="#1E2A6B" />
          <rect x="356" y="424" width="80" height="58" rx="10" fill="#F0B429" />
          <g fill="none" stroke="#14B8A6" stroke-width="26" stroke-linecap="round">
            <path d="M672 392 a52 52 0 0 1 0 104" />
            <path d="M708 360 a92 92 0 0 1 0 168" />
            <path d="M744 330 a132 132 0 0 1 0 228" />
          </g>
        </g>
      </svg>
      <div class="meta">
        <h2>NFC Keyboard Emulator</h2>
        <p class="tag">{{ t("about.tagline") }}</p>
        <p class="ver">{{ t("about.version") }} {{ version }}</p>
      </div>
    </div>

    <button class="link" @click="openUrl('https://noix.dev')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9" />
        <path d="M3 12h18M12 3c2.5 2.5 2.5 15 0 18M12 3c-2.5 2.5-2.5 15 0 18" />
      </svg>
      {{ t("about.website") }}
    </button>

    <div class="card update">
      <button class="btn" :disabled="checking" @click="$emit('recheck')">
        {{ checking ? t("update.checking") : t("update.check") }}
      </button>
      <span class="up-status" :class="{ avail: update && update.available }">
        <template v-if="checking"></template>
        <template v-else-if="update && update.available">
          {{ t("update.available", { version: update.latest }) }}
          <a class="dl" @click="openUrl(update.url)">{{ t("update.download") }}</a>
        </template>
        <template v-else-if="update && update.ok">{{ t("update.upToDate") }}</template>
        <template v-else-if="update && !update.ok">{{ t("update.failed") }}</template>
      </span>
    </div>

    <div class="card app-license">
      <span class="lbl">{{ t("about.appLicense") }}</span>
      <span>{{ t("about.appLicenseNote") }}</span>
    </div>

    <div class="third">
      <h3>{{ t("about.thirdParty") }}</h3>
      <p class="muted">{{ t("about.thirdPartyNote") }}</p>
      <div class="lic-box">
        <p class="grp">{{ t("about.rust") }} · {{ rustLicenses.length }}</p>
        <div v-for="d in rustLicenses" :key="`r-${d.name}-${d.version}`" class="row">
          <span class="n">{{ d.name }}</span>
          <span class="v">{{ d.version }}</span>
          <span class="l">{{ d.license }}</span>
        </div>
        <p class="grp">{{ t("about.npm") }} · {{ npmLicenses.length }}</p>
        <div v-for="d in npmLicenses" :key="`n-${d.name}-${d.version}`" class="row">
          <span class="n">{{ d.name }}</span>
          <span class="v">{{ d.version }}</span>
          <span class="l">{{ d.license }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.about {
  display: grid;
  gap: 16px;
  max-width: 620px;
}
.hero {
  display: flex;
  align-items: center;
  gap: 16px;
}
.logo {
  width: 58px;
  height: 58px;
  border-radius: 13px;
  flex-shrink: 0;
}
.meta h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}
.tag {
  margin: 2px 0 0;
  font-size: 13px;
  color: #5b6478;
}
.ver {
  margin: 4px 0 0;
  font-size: 12px;
  color: #8b93a7;
}
.link {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  width: fit-content;
  padding: 8px 14px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #fff;
  font-size: 14px;
  color: #1e2a6b;
  cursor: pointer;
}
.link:hover {
  border-color: #14b8a6;
  color: #0f766e;
}
.link svg {
  width: 17px;
  height: 17px;
}
.card.update {
  display: flex;
  align-items: center;
  gap: 14px;
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  padding: 14px 16px;
}
.update .btn {
  padding: 8px 13px;
  border: 1px solid #d4d9e6;
  border-radius: 8px;
  background: #f4f5fa;
  font-size: 14px;
  cursor: pointer;
  white-space: nowrap;
}
.update .btn:hover:not(:disabled) {
  background: #e9ecf6;
}
.update .btn:disabled {
  opacity: 0.6;
  cursor: default;
}
.up-status {
  font-size: 13px;
  color: #5b6478;
}
.up-status.avail {
  color: #0f5f53;
  font-weight: 500;
}
.dl {
  color: #14b8a6;
  font-weight: 600;
  cursor: pointer;
  text-decoration: underline;
}
.card.app-license {
  display: flex;
  gap: 12px;
  align-items: baseline;
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  padding: 14px 16px;
  font-size: 14px;
}
.app-license .lbl {
  font-weight: 600;
  color: #5b6478;
}
.third h3 {
  margin: 0 0 4px;
  font-size: 15px;
  font-weight: 600;
}
.muted {
  margin: 0 0 10px;
  font-size: 13px;
  color: #5b6478;
}
.lic-box {
  background: #fff;
  border: 1px solid #e4e7f0;
  border-radius: 12px;
  padding: 8px 4px;
  max-height: 300px;
  overflow: auto;
}
.grp {
  margin: 6px 12px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: #8b96cf;
}
.row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 10px;
  align-items: baseline;
  padding: 4px 12px;
  font-size: 12px;
}
.row:hover {
  background: #f4f5fa;
}
.n {
  color: #1b2030;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.v {
  color: #8b93a7;
  font-family: ui-monospace, Menlo, monospace;
}
.l {
  color: #5b6478;
  text-align: right;
  white-space: nowrap;
}
</style>
