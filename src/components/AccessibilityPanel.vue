<script setup lang="ts">
defineProps<{ ok: boolean }>();
defineEmits<{ fix: [] }>();
</script>

<template>
  <section class="panel">
    <div class="status" :class="ok ? 'ok' : 'warn'">
      <span class="dot"></span>
      <span v-if="ok">Zugriff erteilt – oder auf dieser Plattform nicht erforderlich.</span>
      <span v-else>Bedienungshilfen-Zugriff fehlt – das Tippen in andere Apps funktioniert noch nicht.</span>
    </div>

    <p class="muted">
      Auf macOS braucht die App den Zugriff auf die <strong>Bedienungshilfen</strong>
      (Accessibility API), um Tastatureingaben an andere Programme zu senden.
    </p>

    <ol class="steps">
      <li>Systemeinstellungen → <strong>Datenschutz &amp; Sicherheit</strong> → <strong>Bedienungshilfen</strong></li>
      <li>Liste entsperren und <strong>NFC Keyboard Emulator</strong> aktivieren</li>
      <li>App ggf. neu starten</li>
    </ol>

    <div class="actions">
      <button class="primary" @click="$emit('fix')">Einstellungen öffnen &amp; erneut prüfen</button>
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
