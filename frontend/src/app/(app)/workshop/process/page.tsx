import { Suspense } from "react";

import { WorkshopProcessView } from "@/features/workshop";

export default function WorkshopProcessPage() {
  return (
    <Suspense>
      <WorkshopProcessView />
    </Suspense>
  );
}
