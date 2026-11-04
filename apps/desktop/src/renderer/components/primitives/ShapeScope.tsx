import { createContext, useContext, type ReactNode } from "react";

const ShapeScopeContext = createContext<"amf" | undefined>(undefined);

/** Module-owned geometry follows React portals; shell surfaces keep their host shape. */
export function ShapeScope({ children, value }: { children: ReactNode; value?: "amf" | undefined }) {
  return <ShapeScopeContext value={value}><div className="vua-shape-scope" data-vua-shape-scope={value}>{children}</div></ShapeScopeContext>;
}

export function useShapeScope() { return useContext(ShapeScopeContext); }
