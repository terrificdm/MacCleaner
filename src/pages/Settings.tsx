import { CheckCircle2, FlaskConical, Lock, Plus, RotateCcw, ShieldCheck, X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AppLogo } from "@/components/AppLogo";
import { Page } from "@/components/Page";
import { Badge, Button, Modal, Segmented, Slider, Switch } from "@/components/ui";
import { api } from "@/lib/api";
import type { Language, Theme } from "@/lib/types";
import { shortPath } from "@/lib/format";
import { useStore } from "@/store";

export function SettingsPage() {
  const { t } = useTranslation();
  const s = useStore((st) => st.settings!);
  const perms = useStore((st) => st.perms);
  const update = useStore((st) => st.updateSettings);
  const refreshPerms = useStore((st) => st.refreshPerms);
  const [confirmOff, setConfirmOff] = useState(false);
  const [wl, setWl] = useState("");
  const [root, setRoot] = useState("");
  const [draft, setDraft] = useState({ appUnusedDays: s.appUnusedDays, nodeModulesDays: s.nodeModulesDays });
  const [defaults, setDefaults] = useState<string[]>([]);
  const [version, setVersion] = useState("");
  const [removing, setRemoving] = useState<string | null>(null);
  useEffect(() => {
    api().defaultWhitelist().then(setDefaults).catch(() => {});
    api().appVersion().then(setVersion).catch(() => {});
  }, []);
  const missingDefaults = defaults.filter((d) => !s.whitelist.includes(d));
  const removeWhitelist = (p: string) => update({ whitelist: s.whitelist.filter((x) => x !== p) });

  const addPath = (list: string[], value: string, key: "whitelist" | "nodeModulesRoots", reset: () => void) => {
    const v = value.trim();
    if (!v || !(v.startsWith("/") || v.startsWith("~"))) return;
    update({ [key]: [...list, v] });
    reset();
  };

  return (
    <Page id="settings">
      <div className="h-full overflow-y-auto px-8 pb-10">
        <div className="mx-auto max-w-3xl space-y-4">
          <Section title={t("settings.general")}>
            <Row label={t("settings.language")}>
              <Segmented<Language>
                value={s.language}
                onChange={(v) => update({ language: v })}
                options={[
                  { value: "system", label: t("settings.followSystem") },
                  { value: "zh-CN", label: t("settings.langZh") },
                  { value: "en", label: t("settings.langEn") },
                ]}
              />
            </Row>
            <Row label={t("settings.theme")}>
              <Segmented<Theme>
                value={s.theme}
                onChange={(v) => update({ theme: v })}
                options={[
                  { value: "light", label: t("settings.light") },
                  { value: "dark", label: t("settings.dark") },
                  { value: "system", label: t("settings.followSystem") },
                ]}
              />
            </Row>
          </Section>

          <Section title={t("settings.safety")}>
            <Row
              label={
                <span className="flex items-center gap-2">
                  <FlaskConical className="size-4 text-warn" /> {t("settings.dryRun")}
                </span>
              }
              hint={t("settings.dryRunHint")}
            >
              <Switch checked={s.dryRun} onChange={(v) => (v ? update({ dryRun: true }) : setConfirmOff(true))} />
            </Row>
            <Row
              label={
                <span className="flex items-center gap-2">
                  {perms.fullDiskAccess ? <ShieldCheck className="size-4 text-ok" /> : <Lock className="size-4 text-warn" />}
                  {t("settings.fda")}
                </span>
              }
              hint={perms.fullDiskAccess ? t("perm.granted") : t("perm.missing")}
            >
              <div className="flex gap-2">
                {!perms.fullDiskAccess && (
                  <Button size="sm" variant="outline" onClick={() => api().openFdaSettings()}>
                    {t("perm.open")}
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={refreshPerms}>
                  {t("perm.recheck")}
                </Button>
              </div>
            </Row>
            <div className="px-5 py-4">
              <div className="flex items-center justify-between gap-3">
                <div className="text-[13.5px] font-medium">{t("settings.whitelist")}</div>
                {missingDefaults.length > 0 && (
                  <Button size="sm" variant="ghost" icon={<RotateCcw className="size-3.5" />} onClick={() => update({ whitelist: [...s.whitelist, ...missingDefaults] })}>
                    {t("settings.restoreDefaults", { count: missingDefaults.length })}
                  </Button>
                )}
              </div>
              <div className="mt-0.5 text-[12px] text-muted">{t("settings.whitelistHint")}</div>
              <PathList
                list={[...s.whitelist].sort((a, b) => Number(defaults.includes(b)) - Number(defaults.includes(a)))}
                isDefault={(p) => defaults.includes(p)}
                onRemove={(p) => (defaults.includes(p) ? setRemoving(p) : removeWhitelist(p))}
                value={wl}
                setValue={setWl}
                onAdd={() => addPath(s.whitelist, wl, "whitelist", () => setWl(""))}
                placeholder={t("settings.pathPlaceholder")}
                empty={t("settings.whitelistEmpty")}
              />
            </div>
          </Section>

          <Section title={t("settings.rules")}>
            <Row label={t("settings.appUnusedDays")} hint={t("settings.appUnusedHint")}>
              <SliderValue
                value={draft.appUnusedDays}
                min={14}
                max={730}
                step={1}
                fmt={(v) => t("large.days", { count: v })}
                onChange={(v) => setDraft({ ...draft, appUnusedDays: v })}
                onCommit={(v) => update({ appUnusedDays: v })}
              />
            </Row>
            <Row label={t("settings.nodeModulesDays")} hint={t("settings.nodeModulesHint")}>
              <SliderValue
                value={draft.nodeModulesDays}
                min={7}
                max={730}
                step={1}
                fmt={(v) => t("large.days", { count: v })}
                onChange={(v) => setDraft({ ...draft, nodeModulesDays: v })}
                onCommit={(v) => update({ nodeModulesDays: v })}
              />
            </Row>
            <div className="px-5 py-4">
              <div className="text-[13.5px] font-medium">{t("settings.nodeModulesRoots")}</div>
              <div className="mt-0.5 text-[12px] text-muted">{t("settings.nodeModulesRootsHint")}</div>
              <PathList
                list={s.nodeModulesRoots}
                onRemove={(p) => update({ nodeModulesRoots: s.nodeModulesRoots.filter((x) => x !== p) })}
                value={root}
                setValue={setRoot}
                onAdd={() => addPath(s.nodeModulesRoots, root, "nodeModulesRoots", () => setRoot(""))}
                placeholder={t("settings.pathPlaceholder")}
                empty="—"
              />
            </div>
          </Section>

          <Section title={t("settings.about")}>
            <div className="flex items-center gap-4 px-5 py-4">
              <AppLogo className="size-12" />
              <div>
                <div className="font-semibold">MacCleaner {version}</div>
                <div className="text-[12px] text-muted">{t("settings.aboutText")}</div>
              </div>
            </div>
          </Section>
        </div>
      </div>
      <Modal
        open={removing !== null}
        onClose={() => setRemoving(null)}
        title={t("settings.removeDefaultTitle")}
        description={t("settings.removeDefaultDesc", { path: removing ?? "" })}
        footer={
          <>
            <Button variant="ghost" onClick={() => setRemoving(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                if (removing) removeWhitelist(removing);
                setRemoving(null);
              }}
            >
              {t("settings.removeDefaultConfirm")}
            </Button>
          </>
        }
      />
      <Modal
        open={confirmOff}
        onClose={() => setConfirmOff(false)}
        title={t("settings.dryRunOffTitle")}
        description={t("settings.dryRunOffDesc")}
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmOff(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              icon={<CheckCircle2 className="size-4" />}
              onClick={() => {
                update({ dryRun: false });
                setConfirmOff(false);
              }}
            >
              {t("settings.dryRunOffConfirm")}
            </Button>
          </>
        }
      />
    </Page>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <div className="mb-2 px-1 text-[11px] font-semibold uppercase tracking-wider text-faint">{title}</div>
      <div className="divide-y divide-[var(--line)] overflow-hidden rounded-2xl glass">{children}</div>
    </section>
  );
}

function Row({ label, hint, children }: { label: ReactNode; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6 px-5 py-4">
      <div className="min-w-0">
        <div className="text-[13.5px] font-medium">{label}</div>
        {hint && <div className="mt-0.5 text-[12px] text-muted">{hint}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function SliderValue({
  value, min, max, step, fmt, onChange, onCommit,
}: { value: number; min: number; max: number; step: number; fmt: (v: number) => string; onChange: (v: number) => void; onCommit: (v: number) => void }) {
  return (
    <div className="flex w-64 items-center gap-3">
      <Slider value={value} min={min} max={max} step={step} onChange={onChange} onCommit={onCommit} />
      <span className="w-16 text-right text-[12.5px] font-medium tabular">{fmt(value)}</span>
    </div>
  );
}

function PathList({
  list, onRemove, value, setValue, onAdd, placeholder, empty, isDefault,
}: {
  list: string[];
  isDefault?: (p: string) => boolean;
  onRemove: (p: string) => void;
  value: string;
  setValue: (v: string) => void;
  onAdd: () => void;
  placeholder: string;
  empty: string;
}) {
  return (
    <div className="mt-3 space-y-2">
      {list.length === 0 && <div className="text-[12px] text-faint">{empty}</div>}
      {list.map((p) => (
        <div key={p} className="flex items-center gap-2 rounded-xl bg-card px-3 py-2 text-[12.5px]">
          <span className="min-w-0 flex-1 truncate font-mono selectable">{shortPath(p)}</span>
          {isDefault?.(p) && <DefaultBadge />}
          <button className="rounded-md p-0.5 text-faint hover:bg-card-hover hover:text-danger" onClick={() => onRemove(p)}>
            <X className="size-3.5" />
          </button>
        </div>
      ))}
      <div className="flex gap-2">
        <input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && onAdd()}
          placeholder={placeholder}
          className="h-8 flex-1 rounded-lg border border-line bg-transparent px-3 font-mono text-[12.5px] outline-none placeholder:text-faint focus:border-[var(--accent)]"
        />
        <Button size="sm" variant="outline" icon={<Plus className="size-3.5" />} onClick={onAdd}>
          {""}
        </Button>
      </div>
    </div>
  );
}

function DefaultBadge() {
  const { t } = useTranslation();
  return <Badge>{t("settings.defaultBadge")}</Badge>;
}
