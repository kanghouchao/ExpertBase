import { Suspense } from "react";

import { WorkshopView } from "@/features/workshop";

export default function WorkshopPage() {
  return (
    <Suspense fallback={null}>
      <WorkshopView />
    </Suspense>
  );
}
