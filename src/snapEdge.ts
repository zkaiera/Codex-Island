export type SnapEdge = "top" | "left" | "right" | "floating";

export function toBackendEdge(edge: SnapEdge): "top" | "left" | "right" | null {
  return edge === "floating" ? null : edge;
}
