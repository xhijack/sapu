import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card, CardText, CardTitle } from "@/components/ui/card";
import { aiApi } from "@/lib/aiApi";
import { useErrorMessage } from "@/lib/errors";
import type { AiConfig, AiModeKind, CloudKind } from "@/lib/types";
import { CUSTOM, PRESETS, presetFor } from "./providers";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="grid gap-1 text-sm">
      <span className="font-medium">{label}</span>
      {children}
    </label>
  );
}

const input = "h-10 rounded-lg border border-border bg-surface px-3 text-sm";
const CUSTOM_MODEL = "__custom__";

/** Provider configuration; keys go to the OS keychain (PRD §12.5). */
export function AiSettings() {
  const { t } = useTranslation();
  const errorMessage = useErrorMessage();
  const [cfg, setCfg] = useState<AiConfig | null>(null);
  const [hasKey, setHasKey] = useState(false);
  const [keyHint, setKeyHint] = useState<string | null>(null);
  const [key, setKey] = useState("");
  const [models, setModels] = useState<string[] | null>(null);
  const [customModel, setCustomModel] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    aiApi
      .status()
      .then((s) => {
        if (!live) return;
        setCfg(s.config);
        setHasKey(s.has_key);
        setKeyHint(s.key_hint);
      })
      .catch((e) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const save = async (next: AiConfig) => {
    setCfg(next);
    setError(null);
    try {
      await aiApi.configSet(next);
      setNotice(t("ai.saved"));
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const loadModels = async () => {
    setError(null);
    try {
      const m = await aiApi.models();
      setModels(m);
      setCustomModel(false);
      if (m.length === 0) setNotice(t("ai.no_models"));
      else if (cfg && cfg.mode === "cloud" && !cfg.cloud_model && m[0]) void save({ ...cfg, cloud_model: m[0] });
    } catch (e) {
      setModels([]);
      setError(errorMessage(e));
    }
  };

  const saveKey = async () => {
    setError(null);
    try {
      await aiApi.setApiKey(key);
      const has = key.trim().length > 0;
      setHasKey(has);
      setKeyHint(has ? `…${key.trim().slice(-4)}` : null);
      setKey("");
      setNotice(has ? t("ai.key_saved") : t("ai.saved"));
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const test = async () => {
    setError(null);
    setNotice(null);
    try {
      setNotice(t("ai.test_ok", { model: await aiApi.test() }));
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  if (!cfg) return null;
  return (
    <Card className="grid gap-4">
      <CardTitle>{t("ai.settings_title")}</CardTitle>
      <CardText>{t("ai.settings_body")}</CardText>
      <Field label={t("ai.mode")}>
        <select id="ai-mode" value={cfg.mode} onChange={(e) => void save({ ...cfg, mode: e.target.value as AiModeKind })} className={input}>
          <option value="off">{t("onboarding.ai_off")}</option>
          <option value="local">{t("onboarding.ai_local")}</option>
          <option value="cloud">{t("onboarding.ai_cloud")}</option>
        </select>
      </Field>
      {cfg.mode === "local" && (
        <>
          <Field label={t("ai.local_url")}>
            <input value={cfg.local_url} onChange={(e) => setCfg({ ...cfg, local_url: e.target.value })} onBlur={() => void save(cfg)} className={input} />
          </Field>
          <Field label={t("ai.local_model")}>
            <div className="flex gap-2">
              <select value={cfg.local_model} onChange={(e) => void save({ ...cfg, local_model: e.target.value })} className={`${input} flex-1`}>
                <option value="">{t("ai.model_auto")}</option>
                {(models ?? (cfg.local_model ? [cfg.local_model] : [])).map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
              <Button variant="outline" onClick={() => void loadModels()}>
                {t("ai.refresh_models")}
              </Button>
            </div>
          </Field>
        </>
      )}
      {cfg.mode === "cloud" && (
        <>
          <Field label={t("ai.provider")}>
            <select
              value={presetFor(cfg.cloud_kind, cfg.cloud_url)}
              onChange={(e) => {
                const preset = PRESETS.find((p) => p.id === e.target.value);
                setModels(null);
                if (preset) void save({ ...cfg, cloud_kind: preset.kind, cloud_url: preset.url, cloud_model: "" });
                else void save({ ...cfg, cloud_kind: "openai_compatible" });
              }}
              className={input}
            >
              {PRESETS.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
              <option value={CUSTOM}>{t("ai.provider_custom")}</option>
            </select>
            {PRESETS.find((p) => p.id === presetFor(cfg.cloud_kind, cfg.cloud_url))?.keysUrl && (
              <span className="text-xs text-fg-muted">{t("ai.provider_keys_hint", { url: PRESETS.find((p) => p.id === presetFor(cfg.cloud_kind, cfg.cloud_url))?.keysUrl })}</span>
            )}
          </Field>
          {presetFor(cfg.cloud_kind, cfg.cloud_url) === CUSTOM && (
            <Field label={t("ai.cloud_kind")}>
              <select value={cfg.cloud_kind} onChange={(e) => void save({ ...cfg, cloud_kind: e.target.value as CloudKind })} className={input}>
                <option value="openai_compatible">{t("ai.cloud_openai")}</option>
                <option value="anthropic">{t("ai.cloud_anthropic")}</option>
              </select>
            </Field>
          )}
          <Field label={t("ai.cloud_url")}>
            <input value={cfg.cloud_url} onChange={(e) => setCfg({ ...cfg, cloud_url: e.target.value })} onBlur={() => void save(cfg)} className={input} />
          </Field>
          <Field label={`${t("ai.api_key")} (${hasKey ? t("ai.api_key_set_hint", { tail: keyHint ?? "" }) : t("ai.api_key_missing")})`}>
            <div className="flex gap-2">
              <input type="password" value={key} onChange={(e) => setKey(e.target.value)} className={`${input} flex-1`} autoComplete="off" placeholder={hasKey ? t("ai.api_key_replace") : ""} />
              <Button variant="outline" onClick={() => void saveKey()}>
                {t("ai.save_key")}
              </Button>
            </div>
            <span className="text-xs text-fg-muted">{t("ai.api_key_hint")}</span>
          </Field>
          <Field label={t("ai.cloud_model")}>
            <div className="flex gap-2">
              {models && models.length > 0 && !customModel ? (
                <select
                  value={models.includes(cfg.cloud_model) ? cfg.cloud_model : CUSTOM_MODEL}
                  onChange={(e) => {
                    if (e.target.value === CUSTOM_MODEL) setCustomModel(true);
                    else void save({ ...cfg, cloud_model: e.target.value });
                  }}
                  className={`${input} flex-1`}
                >
                  {!models.includes(cfg.cloud_model) && cfg.cloud_model && <option value={CUSTOM_MODEL}>{cfg.cloud_model}</option>}
                  {models.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                  <option value={CUSTOM_MODEL}>{t("ai.model_custom")}</option>
                </select>
              ) : (
                <input value={cfg.cloud_model} onChange={(e) => setCfg({ ...cfg, cloud_model: e.target.value })} onBlur={() => void save(cfg)} className={`${input} flex-1`} placeholder={t("ai.model_placeholder")} />
              )}
              <Button variant="outline" onClick={() => void loadModels()}>
                {models && models.length > 0 ? t("ai.refresh_models") : t("ai.load_models")}
              </Button>
            </div>
            <span className="text-xs text-fg-muted">
              {models && models.length > 0 && customModel ? (
                <button type="button" className="text-accent-strong underline-offset-2 hover:underline" onClick={() => setCustomModel(false)}>
                  {t("ai.model_pick")}
                </button>
              ) : (
                t("ai.model_hint")
              )}
            </span>
          </Field>
        </>
      )}
      {cfg.mode !== "off" && (
        <>
          <label className="flex items-start gap-3 text-sm">
            <input type="checkbox" checked={cfg.anonymize} onChange={(e) => void save({ ...cfg, anonymize: e.target.checked })} className="mt-0.5 h-4 w-4" />
            <span>{t("ai.anonymize")}</span>
          </label>
          <div>
            <Button variant="outline" size="sm" onClick={() => void test()}>
              {t("ai.test")}
            </Button>
          </div>
        </>
      )}
      {notice && (
        <p role="status" className="text-sm text-fg-muted">
          {notice}
        </p>
      )}
      {error && (
        <p role="alert" className="text-sm text-risk-danger-fg">
          {error}
        </p>
      )}
    </Card>
  );
}
