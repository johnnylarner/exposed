import type { IconName } from "./icon-types";
export type EntityAccent = "member" | "funder" | "neutral";
export interface CategoryWheelOption<Id extends string = string> {
  id: Id;
  label: string;
  icon: IconName;
  accent: EntityAccent;
  path: string;
  rimPath: string;
  left: number;
  top: number;
  width?: number;
  height?: number;
  x: number;
  y: number;
}
export interface RankingRow {
  key: string;
  name: string;
  context: string;
  href?: string;
  amount: string;
  incomplete: boolean;
  width: number;
  negative: boolean;
  accent: EntityAccent;
}
