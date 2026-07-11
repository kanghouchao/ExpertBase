// 共有 API の公開面。消費側（features / entities / widgets）はここからだけ import する。
export * from "./types";
export { agentApi, kbApi, setBackend, workshopApi } from "./backend";
export { fakeBackend } from "./fake";
