<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ReaderStatus, WsStatus } from "../lib/tauri";
import { deliveryErrorText } from "../lib/delivery";
defineProps<{ status: ReaderStatus | null; typing: boolean; ws: WsStatus | null }>();
const { t } = useI18n();

function serverText(ws: WsStatus): string {
  switch (ws.state) {
    case "reconnecting":
      return t("footer.serverReconnecting", { error: deliveryErrorText(t, ws.error) });
    case "connected":
      return t("footer.serverConnected");
    case "connecting":
      return t("footer.serverConnecting");
    case "token_rejected":
      return t("footer.serverTokenRejected");
    default:
      return "";
  }
}
</script>

<template>
  <footer class="bar">
    <span class="item">
      <span class="dot" :class="status?.kind === 'connected' ? 'on' : 'off'"></span>
      {{ t("footer.reader") }}:
      {{
        status?.kind === "connected"
          ? t("footer.connected")
          : status?.kind === "error"
            ? `${t("footer.error")}: ${status.message}`
            : t("footer.disconnected")
      }}
    </span>
    <span class="item">
      <span class="dot" :class="typing ? 'typing' : 'off'"></span>
      {{ t("footer.typing") }}: {{ typing ? t("footer.active") : t("footer.off") }}
    </span>
    <span v-if="ws && ws.state !== 'off'" class="item" data-test="server">
      <span class="dot" :class="ws.state === 'connected' ? 'on' : ws.state === 'token_rejected' ? 'error' : 'typing'"></span>
      {{ t("footer.server") }}: {{ serverText(ws) }}
      <template v-if="ws.requestPending">· {{ t("footer.serverWaiting") }}</template>
    </span>
  </footer>
</template>

<style scoped>
.bar {
  display: flex;
  gap: 18px;
  padding: 8px 18px;
  border-top: 1px solid #e4e7f0;
  font-size: 13px;
  color: #5b6478;
  background: #fff;
}
.item {
  display: flex;
  align-items: center;
  gap: 7px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.dot.on {
  background: #14b8a6;
}
.dot.typing {
  background: #f0b429;
}
.dot.error {
  background: #d9534f;
}
.dot.off {
  background: #c2c8d6;
}
</style>
