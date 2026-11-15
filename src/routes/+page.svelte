<!-- These comments aren't AI generated btw. I actually wrote them because it became confusing which div referred to what lol -->
<!-- I'm not even sure if this will render properly on different screens -->

<div class="w-screen h-screen flex flex-col">
  <!-- // Top Bar -->
  <div class="bg-(--bg) w-full h-[1%]">
    <div>&#x200B;</div>
  </div>

  <div class="flex flex-1 w-full h-[97.5%]">
    <!-- // Left Side Bar -->
    <div class="bg-(--bg) h-full w-[1.75%]">
      <div>&#x200B;</div>
    </div>

    <!-- Empty Desktop Area -->
    <main class="flex-1 h-full bg-transparent"></main>

    <!-- // Right Side Bar -->
    <div class="bg-(--bg) h-full w-[0.5%]">
      <div>&#x200B;</div>
    </div>

    <!-- // Top Left Corner Rounder (yeah lol) -->
    <div
      class="absolute top-2.5 left-[1.75vw] w-6 h-6 pointer-events-none"
      style="background: radial-gradient(circle at 100% 100%, transparent 22px, var(--bg) 24px);"
    ></div>

    <!-- // Top Right Corner Rounder (yeah lol) -->
    <div
      class="absolute top-2.5 right-[0.25vw] w-6 h-6 pointer-events-none"
      style="background: radial-gradient(circle at 0% 90%, transparent 20px, var(--bg) 24px);"
    ></div>

    <!-- // Bottom Left Corner Rounder (yeah lol) -->
    <div
      class="absolute bottom-2.5 left-[1.75vw] w-6 h-6 pointer-events-none"
      style="background: radial-gradient(circle at 100% 0%, transparent 22px, var(--bg) 24px);"
    ></div>

    <!-- // Bottom Right Corner Rounder (yeah lol) -->
    <div
      class="absolute bottom-2.5 right-[0.25vw] w-6 h-6 pointer-events-none"
      style="background: radial-gradient(circle at 0% 0%, transparent 26px, var(--bg) 24px);"
    ></div>

    <!-- // Bottom Bar -->
    <div class="absolute bottom-0 left-auto right-auto bg-(--bg) h-[1%] w-full">
      <div>&#x200B;</div>
    </div>
  </div>
</div>
