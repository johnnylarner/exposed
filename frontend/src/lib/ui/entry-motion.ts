type MotionPreset = "roll-up" | "row" | "bar" | "fade";

type EntryMotionOptions = {
  preset: MotionPreset;
  changeKey?: string;
  initial?: boolean;
  index?: number;
};

const presets = {
  "roll-up": {
    frames: [{ transform: "translateY(100%)" }, { transform: "translateY(0)" }],
    duration: 200,
    easing: "ease-out",
  },
  row: {
    frames: [
      { opacity: 0, transform: "translateY(8px)" },
      { opacity: 1, transform: "translateY(0)" },
    ],
    duration: 300,
    easing: "ease",
  },
  bar: {
    frames: [{ transform: "scaleX(0)" }, { transform: "scaleX(1)" }],
    duration: 420,
    easing: "ease",
  },
  fade: {
    frames: [{ opacity: 0.65 }, { opacity: 1 }],
    duration: 200,
    easing: "ease-out",
  },
} satisfies Record<
  MotionPreset,
  { frames: Keyframe[]; duration: number; easing: string }
>;

export function entryMotion(node: HTMLElement, options: EntryMotionOptions) {
  const preference = matchMedia("(prefers-reduced-motion: reduce)");
  let previousKey = options.changeKey;
  let animation: Animation | undefined;

  function play(next: EntryMotionOptions) {
    animation?.cancel();
    if (preference.matches) return;
    const preset = presets[next.preset];
    animation = node.animate(preset.frames, {
      duration: preset.duration,
      easing: preset.easing,
      delay: (next.index ?? 0) * 35,
      fill: "backwards",
    });
  }

  function cancelReducedMotion() {
    if (preference.matches) animation?.cancel();
  }
  preference.addEventListener("change", cancelReducedMotion);
  if (options.initial) play(options);

  return {
    update(next: EntryMotionOptions) {
      if (next.changeKey === previousKey) return;
      previousKey = next.changeKey;
      play(next);
    },
    destroy() {
      preference.removeEventListener("change", cancelReducedMotion);
      animation?.cancel();
    },
  };
}
