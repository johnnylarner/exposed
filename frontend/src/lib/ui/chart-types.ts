export type PieSlice = {
  share: number;
  path: string;
  color: string;
  label: string;
  value: string;
  detail: string;
};
export type ChartBar = {
  x: number;
  y: number;
  width: number;
  height: number;
  negative: boolean;
  title: string;
  tick?: { x: number; label: string; anchor: "start" | "middle" | "end" };
};
export type SignedBar = {
  label: string;
  value: string;
  width: number;
  negative: boolean;
  note?: string;
};
