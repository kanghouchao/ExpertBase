// knowledge-base エンティティの公開 API。
export {
  useKbStore,
  refreshKbs,
  createAndActivateKb,
  switchKb,
  removeKb,
} from "./model/store";
export type { KbState } from "./model/store";
