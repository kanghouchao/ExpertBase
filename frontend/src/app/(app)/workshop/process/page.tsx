import { Suspense } from "react";

import { WorkshopProcessView } from "./workshop-process-view";

export default function WorkshopProcessPage() {
  return (
    <Suspense>
      <WorkshopProcessView />
    </Suspense>
  );
}
