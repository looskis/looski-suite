<script lang="ts">
  const paths = {
    calendar: 'M4 5h16v16H4z M8 3v4 M16 3v4 M4 10h16 M8 14h3 M14 14h2',
    clock: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18 M12 7v5l3 2',
    people:
      'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2 M16 3a4 4 0 0 1 0 8 M22 21v-2a4 4 0 0 0-3-3.87 M13 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0',
    building:
      'M4 21V3h12v18 M2 21h20 M16 9h4v12 M8 7h4 M8 11h4 M8 15h4 M9 21v-3h2v3',
    tasks: 'M9 5h12 M9 12h12 M9 19h12 M2 5l1 1 2-2 M2 12l1 1 2-2 M2 19l1 1 2-2',
    files:
      'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z M14 2v6h6 M8 13h8 M8 17h5',
    content: 'M4 5h16 M4 10h16 M4 15h10 M4 20h7',
    form: 'M5 3h14a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1 M8 8h8 M8 12h8 M8 16h4',
    board: 'M3 3h18v18H3z M9 3v18 M15 3v18 M5.5 7h1.5 M11 7h2 M17 7h2 M11 11h2',
    book: 'M4 4a2 2 0 0 1 2-2h14v20H6a2 2 0 0 1-2-2z M4 18a2 2 0 0 1 2-2h14 M9 7h7',
    tag: 'M20 13l-7 7a2 2 0 0 1-3 0L2 12V2h10l8 8a2 2 0 0 1 0 3 M7 7h.01',
    grid: 'M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z',
    inbox: 'M4 4h16l2 12v4H2v-4z M2 16h6l2 3h4l2-3h6',
    access: 'M12 22s8-4 8-11V5l-8-3-8 3v6c0 7 8 11 8 11 M9 12l2 2 4-4',
    activity: 'M2 12h5l3-8 4 16 3-8h5',
    export: 'M12 3v12 M8 11l4 4 4-4 M4 16v5h16v-5',
    arrow: 'M4 12h16 M14 6l6 6-6 6',
    logout: 'M9 3H4v18h5 M9 12h13 M18 8l4 4-4 4',
  };
  let { name, size = 18 }: { name: keyof typeof paths; size?: number } =
    $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={paths[name]} /></svg
>
