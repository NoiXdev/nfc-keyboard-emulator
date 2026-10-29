<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ReaderStatus, WsStatus } from "../lib/tauri";
import { deliveryErrorText } from "../lib/delivery";
defineProps<{
  status: ReaderStatus | null;
  typing: boolean;
  ws: WsStatus | null;
  httpEnabled: boolean;
}>();
const { t } = useI18n();

// With the websocket off, the item still says whether scans go out over HTTP.
function serverText(ws: WsStatus | null, httpEnabled: boolean): string {
  if (!ws || ws.state === "off") return httpEnabled ? t("footer.serverHttp") : t("footer.off");
  switch (ws.state) {
    case "reconnecting":
      return t("footer.serverReconnecting", { error: deliveryErrorText(t, ws.error) });
    case "connected":
      return t("footer.serverConnected");
    case "connecting":
      return t("footer.serverConnecting");
    case "token_rejected":
      return t("footer.serverTokenRejected");
  }
}

function serverDot(ws: WsStatus | null, httpEnabled: boolean): string {
  switch (ws?.state ?? "off") {
    case "connected":
      return "on";
    case "token_rejected":
      return "error";
    case "off":
      return httpEnabled ? "on" : "off";
    default:
      return "typing";
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
    <span class="item" data-test="server">
      <span class="dot" :class="serverDot(ws, httpEnabled)"></span>
      {{ t("footer.server") }}: {{ serverText(ws, httpEnabled) }}
      <template v-if="ws?.requestPending">· {{ t("footer.serverWaiting") }}</template>
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
