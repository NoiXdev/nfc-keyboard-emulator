<script setup lang="ts">
import { NAV_GROUPS, type NavView } from "../lib/nav";

defineProps<{ active: NavView; accessibilityOk: boolean }>();
defineEmits<{ navigate: [view: NavView] }>();
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
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
      <div class="brand-name">
        <span>NFC Keyboard</span>
        <span>Emulator</span>
      </div>
    </div>

    <nav>
      <div v-for="group in NAV_GROUPS" :key="group.label" class="group">
        <p class="group-label">{{ group.label }}</p>
        <button
          v-for="item in group.items"
          :key="item.key"
          class="item"
          :class="{ active: active === item.key }"
          @click="$emit('navigate', item.key)"
        >
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <template v-if="item.key === 'scanner'">
              <rect x="3" y="6" width="12" height="12" rx="2" />
              <path d="M18 9a4 4 0 0 1 0 6" />
            </template>
            <template v-else-if="item.key === 'log'">
              <line x1="4" y1="7" x2="20" y2="7" />
              <line x1="4" y1="12" x2="20" y2="12" />
              <line x1="4" y1="17" x2="14" y2="17" />
            </template>
            <template v-else-if="item.key === 'options'">
              <line x1="4" y1="8" x2="20" y2="8" />
              <circle cx="9" cy="8" r="2.2" />
              <line x1="4" y1="16" x2="20" y2="16" />
              <circle cx="15" cy="16" r="2.2" />
            </template>
            <template v-else>
              <circle cx="12" cy="5" r="1.4" />
              <path d="M5 9h14M12 9v6M9 21l3-6 3 6" />
            </template>
          </svg>
          <span>{{ item.label }}</span>
          <span v-if="item.key === 'accessibility' && !accessibilityOk" class="badge" title="Bedienungshilfen fehlen"></span>
        </button>
      </div>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 210px;
  flex-shrink: 0;
  background: #1e2a6b;
  color: #c9d0f0;
  display: flex;
  flex-direction: column;
  padding: 18px 12px;
  gap: 22px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 6px;
}
.logo {
  width: 38px;
  height: 38px;
  border-radius: 9px;
  flex-shrink: 0;
}
.brand-name {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
  font-weight: 600;
  font-size: 14px;
  color: #fff;
}
nav {
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.group-label {
  margin: 0 0 6px 8px;
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #8b96cf;
}
.item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: none;
  border-left: 3px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: #c9d0f0;
  font-size: 14px;
  text-align: left;
  cursor: pointer;
}
.item:hover {
  background: rgba(255, 255, 255, 0.07);
  color: #fff;
}
.item.active {
  background: rgba(20, 184, 166, 0.16);
  border-left-color: #14b8a6;
  color: #fff;
}
.icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}
.badge {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f97066;
  margin-left: auto;
}
</style>
