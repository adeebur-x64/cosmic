<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  function syncLayout() {
    // 1. Sync interactive regions for selective click-through
    const elements = document.querySelectorAll(".interactive");
    const regions = Array.from(elements).map((el) => {
      const r = el.getBoundingClientRect();
      return {
        x: r.left,
        y: r.top,
        width: r.width,
        height: r.height,
      };
    });
    invoke("update_interactive_regions", { regions }).catch(() => {});

    // 2. Sync Windows desktop work area so other windows only stay inside <main>
    const mainEl = document.querySelector("main");
    if (mainEl) {
      const dpr = window.devicePixelRatio || 1;
      const mRect = mainEl.getBoundingClientRect();
      const left = Math.round(mRect.left * dpr);
      const top = Math.round(mRect.top * dpr);
      const right = Math.round(mRect.right * dpr);
      const bottom = Math.round(mRect.bottom * dpr);

      invoke("set_work_area", {
        rect: { left, top, right, bottom },
      }).catch(() => {});
    }
  }

  onMount(() => {
    syncLayout();
    setTimeout(syncLayout, 100);
    setTimeout(syncLayout, 500);
    window.addEventListener("resize", syncLayout);
    const observer = new ResizeObserver(syncLayout);
    document
      .querySelectorAll(".interactive, main")
      .forEach((el) => observer.observe(el));

    return () => {
      window.removeEventListener("resize", syncLayout);
      observer.disconnect();
    };
  });

  // Getting WiFi Status (Connected or Not + SSID)
  type WifiInfo = {
    status: string;
    ssid: string | null;
    error_code: number | null;
  };

  let wifi: WifiInfo | null = $state(null);

  async function refreshWiFi() {
    try {
      wifi = await invoke<WifiInfo>("get_wifi_status");
    } catch (e) {
      console.error("Failed to get WiFi status", e);
    }
  }

  // Getting Bluetooth Status (Status + Connected Device Name)
  type BluetoothInfo = {
    status: string;
    device_name: string | null;
    error_code: number | null;
  };

  let bluetooth: BluetoothInfo | null = $state(null);

  async function refreshBluetooth() {
    try {
      bluetooth = await invoke<BluetoothInfo>("get_bluetooth_status");
    } catch (e) {
      console.error("Failed to get Bluetooth status", e);
    }
  }

  onMount(() => {
    refreshWiFi();
    refreshBluetooth();
  });
</script>

<div class="w-screen h-screen flex flex-col overflow-hidden select-none">
  <!-- // Top Bar -->
  <div class="bg-(--bg) w-full h-[1%] interactive top-bar">
    <div>&#x200B;</div>
  </div>

  <div class="flex flex-1 w-full min-h-0">
    <!-- // Left Side Bar -->
    <div
      class="bg-(--bg) h-full w-[2.25%] flex flex-col justify-between items-center interactive left-bar"
    >
      <!-- Top Logo -->
      <div
        class="relative group cursor-pointer flex items-center justify-center"
      >
        <div class="w-full flex items-center justify-center pt-2">
          <img
            src="/win10/pride_win10.svg"
            alt="logo"
            class="w-7 h-7 object-contain"
          />
        </div>
        <div
          class="absolute left-full ml-3 top-1/2 -translate-y-1/2 opacity-0 scale-95 pointer-events-none group-hover:opacity-100 group-hover:scale-100 transition-all duration-150 flex flex-col gap-1 bg-[#1e1e1e]/95 border border-white/10 rounded-xl p-3 shadow-2xl min-w-20 z-50 select-none whitespace-nowrap"
        >
          <div class="flex items-center justify-between">
            <span class="text-[12px] font-semibold text-white">Start Menu</span>
          </div>
        </div>
      </div>

      <!-- Bottom Nav Icons -->
      <div
        class="mt-auto w-full flex flex-col gap-5 items-center justify-center z-10 mb-8"
      >
        <div
          class="flex flex-col gap-3 bg-[#2e2e2e] p-1.5 rounded-3xl items-center"
        >
          <!-- Wi-Fi -->
          <div
            class="relative group cursor-pointer flex items-center justify-center"
            onmouseenter={refreshWiFi}
            role="button"
            tabindex="0"
          >
            <div
              class="w-5 h-5 bg-gray-100 group-hover:bg-white transition-colors hover:bg-red-500"
              style="-webkit-mask: url('navBar/wifi.svg') no-repeat center / contain; mask: url('navBar/wifi.svg') no-repeat center / contain;"
            ></div>

            <!-- Wi-Fi Hover Card -->
            <div
              class="absolute left-full ml-3 top-1/2 -translate-y-1/2 opacity-0 scale-95 pointer-events-none group-hover:opacity-100 group-hover:scale-100 transition-all duration-150 flex flex-col gap-1 bg-[#1e1e1e]/95 border border-white/10 rounded-xl p-3 shadow-2xl min-w-44 z-50 select-none whitespace-nowrap"
            >
              <div class="flex items-center justify-between gap-3">
                <span class="text-[14px] font-semibold text-white">WiFi</span>
                <span class="text-[10px] text-emerald-400 font-medium"
                  >{wifi?.status ?? "Unknown"}</span
                >
              </div>
              <div class="text-[12px] text-gray-300 font-medium">
                {wifi?.ssid ?? "Not Connected"}
              </div>
            </div>
          </div>

          <!-- Bluetooth -->
          <div
            class="relative group cursor-pointer flex items-center justify-center"
            onmouseenter={refreshBluetooth}
            role="button"
            tabindex="0"
          >
            <div
              class="w-5 h-5 bg-gray-100 group-hover:bg-white transition-colors hover:bg-red-500"
              style="-webkit-mask: url('navBar/bluetooth.svg') no-repeat center / contain; mask: url('navBar/bluetooth.svg') no-repeat center / contain;"
            ></div>

            <!-- Bluetooth Hover Card -->
            <div
              class="absolute left-full ml-3 top-1/2 -translate-y-1/2 opacity-0 scale-95 pointer-events-none group-hover:opacity-100 group-hover:scale-100 transition-all duration-150 flex flex-col gap-1 bg-[#1e1e1e]/95 border border-white/10 rounded-xl p-3 shadow-2xl min-w-44 z-50 select-none whitespace-nowrap"
            >
              <div class="flex items-center justify-between gap-3">
                <span class="text-[14px] font-semibold text-white"
                  >Bluetooth</span
                >
                <span class="text-[10px] text-blue-400 font-medium"
                  >{bluetooth?.status ?? "Off"}</span
                >
              </div>
              <div class="text-[12px] text-gray-300 font-medium">
                {bluetooth?.device_name ?? "No Device Connected"}
              </div>
            </div>
          </div>

          <!-- Battery -->
          <div
            class="relative group cursor-pointer flex items-center justify-center"
          >
            <div
              class="w-5 h-5 bg-gray-100 group-hover:bg-white transition-colors hover:bg-red-500"
              style="-webkit-mask: url('navBar/battery.svg') no-repeat center / contain; mask: url('navBar/battery.svg') no-repeat center / contain;"
            ></div>

            <!-- Battery Hover Card -->
            <div
              class="absolute left-full ml-3 top-1/2 -translate-y-1/2 opacity-0 scale-95 pointer-events-none group-hover:opacity-100 group-hover:scale-100 transition-all duration-150 flex flex-col gap-1 bg-[#1e1e1e]/95 border border-white/10 rounded-xl p-3 shadow-2xl min-w-44 z-50 select-none whitespace-nowrap"
            >
              <div class="flex items-center justify-between gap-3">
                <span class="text-xs font-semibold text-white">Battery</span>
                <span class="text-[10px] text-emerald-400 font-medium"
                  >Charging</span
                >
              </div>
              <div class="text-base font-bold text-white leading-tight">
                85%
              </div>
            </div>
          </div>
        </div>

        <!-- Power Off Button -->
        <div
          class="relative group cursor-pointer flex items-center justify-center"
        >
          <div
            class="w-5 h-5 bg-red-300 group-hover:bg-red-500 transition-colors"
            style="-webkit-mask: url('navBar/power_off.svg') no-repeat center / contain; mask: url('navBar/power_off.svg') no-repeat center / contain;"
          ></div>
          <!-- Power Hover Card -->
          <div
            class="absolute left-full ml-3 top-1/2 -translate-y-1/2 opacity-0 scale-95 pointer-events-none group-hover:opacity-100 group-hover:scale-100 transition-all duration-150 flex flex-col gap-1 bg-[#1e1e1e]/95 border border-white/10 rounded-xl px-3 py-2 shadow-2xl min-w-28 z-50 select-none whitespace-nowrap"
          >
            <span class="text-xs font-semibold text-red-400">Power Options</span
            >
          </div>
        </div>
      </div>
    </div>

    <!-- Empty Desktop Area -->
    <main
      class="relative flex items-center justify-center flex-1 h-full bg-transparent"
    >
      <!-- // Top Left Corner Rounder -->
      <div
        class="absolute top-0 left-0 w-6 h-6 pointer-events-none z-50"
        style="background: radial-gradient(circle at 100% 100%, transparent 22px, var(--bg) 24px);"
      ></div>

      <!-- // Top Right Corner Rounder -->
      <div
        class="absolute top-0 right-0 w-6 h-6 pointer-events-none z-50"
        style="background: radial-gradient(circle at 0% 100%, transparent 22px, var(--bg) 24px);"
      ></div>

      <!-- // Bottom Left Corner Rounder -->
      <div
        class="absolute bottom-0 left-0 w-6 h-6 pointer-events-none z-50"
        style="background: radial-gradient(circle at 100% 0%, transparent 22px, var(--bg) 24px);"
      ></div>

      <!-- // Bottom Right Corner Rounder -->
      <div
        class="absolute bottom-0 right-0 w-6 h-6 pointer-events-none z-50"
        style="background: radial-gradient(circle at 0% 0%, transparent 24px, var(--bg) 24px);"
      ></div>
    </main>

    <!-- // Right Side Bar -->
    <div class="bg-(--bg) h-full w-[0.5%] min-w-[8px] interactive right-bar">
      <div>&#x200B;</div>
    </div>
  </div>

  <!-- // Bottom Bar -->
  <div class="bg-(--bg) h-[1%] min-h-[10px] w-full interactive bottom-bar">
    <div>&#x200B;</div>
  </div>
</div>
