import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import { Markdown } from "./markdown";

test("Markdown renders inline LaTeX with KaTeX", () => {
  const html = renderToStaticMarkup(<Markdown>{"$\\rightarrow$"}</Markdown>);

  expect(html).toContain('class="katex"');
});
