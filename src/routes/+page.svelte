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

    // 2. Sync Windows desktop work area so other windows only stay inside the empty area
    const topBar = document.querySelector(".top-bar");
    const bottomBar = document.querySelector(".bottom-bar");
    const leftBar = document.querySelector(".left-bar");
    const rightBar = document.querySelector(".right-bar");

    if (topBar && bottomBar && leftBar && rightBar) {
      const dpr = window.devicePixelRatio || 1;
      const lRect = leftBar.getBoundingClientRect();
      const tRect = topBar.getBoundingClientRect();
      const rRect = rightBar.getBoundingClientRect();
      const bRect = bottomBar.getBoundingClientRect();

      const left = Math.round(lRect.right * dpr);
      const top = Math.round(tRect.bottom * dpr);
      const rightVal = rRect.left > lRect.right ? rRect.left : (window.innerWidth - Math.max(rRect.width, 10));
      const bottomVal = bRect.top > tRect.bottom ? bRect.top : (window.innerHeight - Math.max(bRect.height, 11));

      const right = Math.round(rightVal * dpr);
      const bottom = Math.round(bottomVal * dpr);

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
      invoke("restore_work_area").catch(() => {});
    };
  });
</script>

<!-- These comments aren't AI generated btw. I actually wrote them because it became confusing which div referred to what lol -->
<!-- I'm not even sure if this will render properly on different screens -->

<div class="w-screen h-screen flex flex-col">
  <!-- // Top Bar -->
  <div class="bg-(--bg) w-full h-[1%] interactive top-bar">
    <div>&#x200B;</div>
  </div>

  <div class="flex flex-1 w-full min-h-0">
    <!-- // Left Side Bar -->
    <div
      class="bg-(--bg) h-full w-[2%] flex flex-col justify-between items-center interactive left-bar"
    >
      <!-- Top Logo -->
      <div class="w-full flex items-center justify-center pt-2">
        <img
          src="/win10/pride_win10.svg"
          alt="logo"
          class="w-7 h-7 object-contain"
        />
      </div>

      <!-- Bottom Nav Icons -->
      <div
        class="mt-auto w-full flex flex-col gap-5 items-center justify-center z-10 mb-8"
      >
        <div
          class="flex flex-col gap-3 bg-[#2e2e2e] p-1.5 rounded-3xl items-center"
        >
          <div
            class="w-5 h-5 bg-gray-100"
            style="-webkit-mask: url('navBar/wifi.svg') no-repeat center / contain; mask: url('navBar/wifi.svg') no-repeat center / contain;"
          ></div>
          <div
            class="w-5 h-5 bg-gray-100"
            style="-webkit-mask: url('navBar/bluetooth.svg') no-repeat center / contain; mask: url('navBar/bluetooth.svg') no-repeat center / contain;"
          ></div>
          <div
            class="w-5 h-5 bg-gray-100"
            style="-webkit-mask: url('navBar/battery.svg') no-repeat center / contain; mask: url('navBar/battery.svg') no-repeat center / contain;"
          ></div>
        </div>
        <div
          class="w-5 h-5 bg-red-400"
          style="-webkit-mask: url('navBar/power_off.svg') no-repeat center / contain; mask: url('navBar/power_off.svg') no-repeat center / contain;"
        ></div>
      </div>
    </div>

    <!-- Empty Desktop Area -->
    <main
      class="flex items-center justify-center flex-1 h-full bg-transparent pb-3"
    ></main>

    <!-- // Right Side Bar -->
    <div class="bg-(--bg) h-full w-[0.5%] min-w-[8px] interactive right-bar">
      <div>&#x200B;</div>
    </div>

    <!-- // Top Left Corner Rounder (yeah lol) -->
    <div
      class="absolute top-2.5 left-[1.75vw] w-6 h-6 pointer-events-none z-50"
      style="background: radial-gradient(circle at 100% 100%, transparent 22px, var(--bg) 24px);"
    ></div>

    <!-- // Top Right Corner Rounder (yeah lol) -->
    <div
      class="absolute top-2.5 right-[0.25vw] w-6 h-6 pointer-events-none z-50"
      style="background: radial-gradient(circle at 0% 90%, transparent 20px, var(--bg) 24px);"
    ></div>

    <!-- // Bottom Left Corner Rounder (yeah lol) -->
    <div
      class="absolute bottom-2.5 left-[1.75vw] w-6 h-6 pointer-events-none z-50"
      style="background: radial-gradient(circle at 100% 0%, transparent 22px, var(--bg) 24px);"
    ></div>

    <!-- // Bottom Right Corner Rounder (yeah lol) -->
    <div
      class="absolute bottom-2.5 right-[0.25vw] w-6 h-6 pointer-events-none z-50"
      style="background: radial-gradient(circle at 0% 0%, transparent 26px, var(--bg) 24px);"
    ></div>

    <!-- // Bottom Bar -->
    <div
      class="absolute bottom-0 left-auto right-auto bg-(--bg) h-[1%] min-h-[10px] w-full interactive bottom-bar"
    >
      <div>&#x200B;</div>
    </div>
  </div>
</div>
