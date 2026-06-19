<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import {
  api, onScan, onReadersChanged, onReaderStatus,
  type Config, type FormatConfig, type ReaderStatus, type ScanRecord,
} from "./lib/tauri";
import { pushScan } from "./lib/scanLog";
import ReaderSelect from "./components/ReaderSelect.vue";
import TypingToggle from "./components/TypingToggle.vue";
import FormatSettings from "./components/FormatSettings.vue";
import ScanLog from "./components/ScanLog.vue";
import StatusBar from "./components/StatusBar.vue";
import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";

const config = ref<Config | null>(null);
const readers = ref<string[]>([]);
const scans = ref<ScanRecord[]>([]);
const status = ref<ReaderStatus | null>(null);
const typing = ref(false);
const accessibilityOk = ref(true);
const unlisten: Array<() => void> = [];

const autostart = ref(false);
const startMinimized = ref(false);

onMounted(async () => {
  config.value = await api.getConfig();
  typing.value = config.value.typingEnabledOnStart;
  readers.value = await api.listReaders();
  accessibilityOk.value = await api.checkAccessibility();
  unlisten.push(await onScan((r) => { scans.value = pushScan(scans.value, r, config.value!.logRetention); }));
  unlisten.push(await onReadersChanged((r) => { readers.value = r; }));
  unlisten.push(await onReaderStatus((s) => { status.value = s; }));
  autostart.value = await isAutostartEnabled();
  startMinimized.value = config.value!.startMinimized;
});
onUnmounted(() => unlisten.forEach((fn) => fn()));

function selectReader(name: string | null) { config.value!.selectedReader = name; api.selectReader(name); }
function toggleTyping(value: boolean) { typing.value = value; api.setTypingEnabled(value); }
function changeFormat(format: FormatConfig) { config.value!.format = format; api.updateFormat(format); }
async function fixAccessibility() { await api.openAccessibilitySettings(); accessibilityOk.value = await api.checkAccessibility(); }
async function exportCsv() {
  await api.exportLogCsv();
}

async function toggleAutostart(v: boolean) { if (v) await enableAutostart(); else await disableAutostart(); autostart.value = v; }
function toggleStartMinimized(v: boolean) { startMinimized.value = v; api.setStartMinimized(v); }
</script>

<template>
  <main v-if="config">
    <h1>NFC-Keyboard-Emulator</h1>
    <ReaderSelect :readers="readers" :selected="config.selectedReader" @select="selectReader" @rescan="api.rescanReaders()" />
    <TypingToggle :enabled="typing" @toggle="toggleTyping" />
    <FormatSettings :format="config.format" @change="changeFormat" />
    <ScanLog :scans="scans" @export="exportCsv" />
    <section class="options">
      <h2>Optionen</h2>
      <label>
        <input type="checkbox" :checked="autostart" @change="toggleAutostart(($event.target as HTMLInputElement).checked)" />
        Autostart beim Systemstart
      </label>
      <label>
        <input type="checkbox" :checked="startMinimized" @change="toggleStartMinimized(($event.target as HTMLInputElement).checked)" />
        Minimiert starten
      </label>
    </section>
    <StatusBar :status="status" :accessibility-ok="accessibilityOk" :typing="typing" @fix-accessibility="fixAccessibility" />
  </main>
</template>

<style>
main { font-family: system-ui, sans-serif; padding: 16px; display: grid; gap: 16px; max-width: 820px; margin: 0 auto; }
</style>

<style scoped>
.options { display: grid; gap: 8px; }
.options label { display: flex; gap: 8px; align-items: center; font-size: 14px; }
</style>
