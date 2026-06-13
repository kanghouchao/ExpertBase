"use client";

import { Menu } from "@base-ui/react/menu";

import { Icon } from "@/components/eb/icon";
import { useI18n } from "@/components/providers";
import { switchKb, useKbStore } from "@/lib/kb/store";

export function KbSwitcher({ onAdd }: { onAdd: () => void }) {
  const { t } = useI18n();
  const { kbs, active } = useKbStore();

  // ブラウザ単体実行などでナレッジベースが取得できない場合は非表示
  if (kbs.length === 0) return null;
  const current = active ?? kbs[0];

  return (
    <Menu.Root modal={false}>
      <div className="relative mt-3.5">
        <Menu.Portal>
          <Menu.Positioner side="top" align="start" sideOffset={8} className="z-50">
            <Menu.Popup
              finalFocus
              className="w-(--anchor-width) rounded-[12px] border border-line bg-surface p-1.5 shadow-(--shadow-lg) outline-none"
            >
              <Menu.Group>
                <Menu.GroupLabel className="px-2.5 pt-1.5 pb-1.25 font-mono text-[10px] font-semibold tracking-[0.12em] text-ink-faint uppercase">
                  {t("kb.switch")}
                </Menu.GroupLabel>
                <Menu.RadioGroup
                  value={current.path}
                  onValueChange={(value) => void switchKb(String(value))}
                >
                  {kbs.map((kb) => {
                    const on = kb.path === current.path;
                    return (
                      <Menu.RadioItem
                        key={kb.path}
                        value={kb.path}
                        closeOnClick
                        label={kb.name}
                        className="flex w-full cursor-default items-center gap-2.5 rounded-[9px] px-2.5 py-2 text-left outline-none transition-colors hover:bg-surface-2 data-highlighted:bg-surface-2 data-checked:bg-surface-2"
                      >
                        <span className="grid size-7 flex-none place-items-center rounded-lg bg-brand text-white">
                          <Icon name="book" size={15} />
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-[13px] font-semibold">
                            {kb.name}
                          </span>
                          <span className="block truncate font-mono text-[10.5px] text-ink-faint">
                            {kb.path}
                          </span>
                        </span>
                        {on && <Icon name="check" size={15} className="flex-none text-brand" />}
                      </Menu.RadioItem>
                    );
                  })}
                </Menu.RadioGroup>
              </Menu.Group>
              <div className="mx-2 my-1.25 h-px bg-line" />
              <Menu.Item
                closeOnClick
                onClick={onAdd}
                className="flex w-full cursor-default items-center gap-2.5 rounded-[9px] px-2.5 py-2 font-semibold text-brand outline-none transition-colors hover:bg-brand-wash data-highlighted:bg-brand-wash"
              >
                <span className="grid size-7 flex-none place-items-center rounded-lg border border-dashed border-brand-soft">
                  <Icon name="plus" size={15} />
                </span>
                <span className="text-[13px]">{t("kb.add")}</span>
              </Menu.Item>
            </Menu.Popup>
          </Menu.Positioner>
        </Menu.Portal>
        <Menu.Trigger className="group flex w-full items-center gap-2.75 rounded-[12px] border border-line bg-surface px-3 py-2.5 text-left shadow-(--shadow-sm) outline-none transition-shadow focus-visible:ring-3 focus-visible:ring-ring/50 data-open:shadow-(--shadow-md)">
          <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-brand text-white">
            <Icon name="book" size={17} />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13px] font-bold">{current.name}</span>
            <span className="block truncate font-mono text-[10.5px] text-ink-faint">
              {current.path}
            </span>
          </span>
          <Icon
            name="chevD"
            size={15}
            className="flex-none text-ink-muted transition-transform group-data-open:rotate-180"
          />
        </Menu.Trigger>
      </div>
    </Menu.Root>
  );
}
