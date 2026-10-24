import { Suspense, use, useCallback, useRef, useState } from 'react';
import { trackDownload, trackScriptsOff } from './analytics';
import { CAMERAS, MODESET_TOKEN, PREVIEW_TARGET, type Camera } from './cameras';
import { loadCatalog, siteUrl, type ImportedEffect } from './catalog';
import { EffectsSection } from './components/EffectsSection';
import { ErrorBoundary } from './components/ErrorBoundary';
import { RichText } from './components/RichText';
import { loadFonts } from './fonts';
import { applyLanguage, initialLanguage, LANGUAGES, STRINGS, type Language } from './i18n';
import { ALL_MODULES, MODULES, planModules, type ModuleKey } from './modules';
import { buildPackage, buildScriptsOff, prefetchPackage, save } from './package';
import { resolveSelection, type Selection } from './selection';

const REPOSITORY = 'https://github.com/ahonn/grkit';
/** The Tally form for feature requests. A plain link, so the page loads nothing from Tally. */
const FEATURE_REQUEST_FORM = 'https://tally.so/r/Z9jRgB';
const DONATE = 'https://buymeacoffee.com/yuexunjiang';

/** A guide in `resources/` in the page language, or in English. */
function guideUrl(name: 'RECOVERY' | 'DUMP', language: Language): string {
  return `${REPOSITORY}/blob/main/resources/${name}${language === 'en' ? '' : `.${language}`}.md`;
}

type Build =
  | { state: 'idle' }
  | { state: 'building'; received: number }
  | { state: 'ready' }
  | { state: 'failed' };

/** Megabytes with one decimal, as the progress text shows them. */
const megabytes = (bytes: number) => (bytes / 1_000_000).toFixed(1);

export function App() {
  const [language, setLanguage] = useState<Language>(initialLanguage);
  const [cameraKey, setCameraKey] = useState(CAMERAS[0]!.key);
  const [selection, setSelection] = useState<Selection>(null);
  const [enabled, setEnabled] = useState<ReadonlySet<ModuleKey>>(ALL_MODULES);
  // Effects fitted from LUTs on this page. They are not stored, so a reload removes them.
  const [imported, setImported] = useState<readonly ImportedEffect[]>([]);
  const [build, setBuild] = useState<Build>({ state: 'idle' });
  // A retry of a failed catalog request clears the error boundaries that use the catalog.
  const [catalogAttempt, setCatalogAttempt] = useState(0);
  // A newer build or a changed selection makes the result of an older build stale.
  const buildRun = useRef(0);

  const t = STRINGS[language];
  const camera = CAMERAS.find((item) => item.key === cameraKey) ?? CAMERAS[0]!;
  const support = camera.support;
  const targets = support?.targets;
  const available = support?.modules ?? [];
  // Before the first change, every catalog effect is checked, so the count is not 0.
  const plan = planModules(enabled, selection?.size ?? 1, available, support?.excluded);

  function prefetch() {
    if (targets) prefetchPackage(targets, camera.id, language);
  }

  // When the effects come into view, the package files start to download, so Download is quick
  // on a slow connection. A change of camera or language observes again for the new files.
  const prefetchInView = useCallback(
    (section: HTMLElement | null) => {
      if (!section || !targets) return;
      const observer = new IntersectionObserver(
        (entries) => {
          if (entries.some((entry) => entry.isIntersecting)) {
            observer.disconnect();
            prefetchPackage(targets, camera.id, language);
          }
        },
        { rootMargin: '200px 0px' },
      );
      observer.observe(section);
      return () => observer.disconnect();
    },
    [targets, camera.id, language],
  );

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next);
    setLanguage(next);
  }

  function resetBuild() {
    buildRun.current += 1;
    setBuild({ state: 'idle' });
  }

  function chooseCamera(key: Camera['key']) {
    setCameraKey(key);
    resetBuild();
  }

  function changeSelection(next: ReadonlySet<string>) {
    setSelection(next);
    resetBuild();
  }

  // The effects section checks an import (`selectImport`).
  function addImported(effect: ImportedEffect) {
    setImported((current) => [...current, effect]);
    resetBuild();
  }

  function removeImported(effect: ImportedEffect) {
    setImported((current) => current.filter((item) => item !== effect));
    if (selection?.has(effect.id)) {
      const next = new Set(selection);
      next.delete(effect.id);
      changeSelection(next);
    } else {
      resetBuild();
    }
  }

  function toggleModule(key: ModuleKey) {
    const next = new Set(enabled);
    if (!next.delete(key)) {
      next.add(key);
    }
    setEnabled(next);
    resetBuild();
  }

  const [scriptsOff, setScriptsOff] = useState<'idle' | 'building' | 'failed'>('idle');

  async function downloadScriptsOff() {
    setScriptsOff('building');
    try {
      save(await buildScriptsOff(camera.id), 'GRKit-scripts-off.zip');
      trackScriptsOff(camera);
      setScriptsOff('idle');
    } catch (error) {
      console.error(error);
      setScriptsOff('failed');
    }
  }

  async function download() {
    if (!targets || plan.visible.length === 0) {
      return;
    }
    buildRun.current += 1;
    const run = buildRun.current;
    setBuild({ state: 'building', received: 0 });
    try {
      // The builds of a camera have the same effects, so the first build's catalog serves the
      // package. Imported effects do not depend on the target.
      const catalog = (await loadCatalog(targets[0]!)).effects;
      const checked = resolveSelection(
        selection,
        catalog.map((effect) => effect.id),
      );
      const effects = plan.includeEffects
        ? [...catalog, ...imported].filter((effect) => checked.has(effect.id))
        : [];
      const blob = await buildPackage(
        targets,
        camera.id,
        effects,
        plan.omitted,
        language,
        (received) => {
          if (run === buildRun.current) setBuild({ state: 'building', received });
        },
      );
      if (run === buildRun.current) {
        save(blob, 'GRKit.zip');
        trackDownload(camera, plan, effects.length, language);
        setBuild({ state: 'ready' });
      }
    } catch (error) {
      console.error(error);
      if (run === buildRun.current) {
        setBuild({ state: 'failed' });
      }
    }
  }

  return (
    <div className="page">
      <div className="languages">
        <a className="github" href={REPOSITORY} aria-label="GitHub" title="GitHub">
          <svg viewBox="0 0 16 16" width="22" height="22" aria-hidden="true" focusable="false">
            <path
              fill="currentColor"
              d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"
            />
          </svg>
        </a>
        <div role="group" aria-label={t.language}>
          {LANGUAGES.map((item) => (
            <button
              key={item.code}
              type="button"
              lang={item.code}
              className="button"
              aria-pressed={item.code === language}
              onClick={() => chooseLanguage(item.code)}
            >
              {item.label}
            </button>
          ))}
        </div>
        <span className="language-select">
          <select
            className="button"
            aria-label={t.language}
            value={language}
            onChange={(event) => chooseLanguage(event.target.value as Language)}
          >
            {LANGUAGES.map((item) => (
              <option key={item.code} value={item.code} lang={item.code}>
                {item.label}
              </option>
            ))}
          </select>
        </span>
      </div>

      <header className="masthead">
        <h1>
          GRK<span className="dot">i</span>t
        </h1>
        <p className="lead">{t.lead}</p>
      </header>

      <main>
        <section className="section" aria-labelledby="camera-label">
          <h2 id="camera-label">{t.camera}</h2>
          <div className="section-body">
            <div role="radiogroup" aria-labelledby="camera-label" className="cameras">
              {CAMERAS.map((item) => (
                <label key={item.key} className="camera">
                  <input
                    type="radio"
                    name="camera"
                    checked={item.key === camera.key}
                    disabled={!item.support}
                    onChange={() => chooseCamera(item.key)}
                  />
                  <span className="camera-text">
                    <span className="camera-name">{item.name}</span>
                    {item.support && (
                      <span className="camera-firmware">
                        {t.firmware} {item.support.firmware}
                      </span>
                    )}
                    <span className="camera-status">
                      <span
                        className={
                          !item.support
                            ? 'badge'
                            : item.support.experimental
                              ? 'badge experimental'
                              : 'badge supported'
                        }
                      >
                        {!item.support
                          ? t.planned
                          : item.support.experimental
                            ? t.experimental
                            : t.supported}
                      </span>
                      {item.support?.excluded?.includes('imageControl') && (
                        <span className="camera-firmware">{t.noEffects}</span>
                      )}
                    </span>
                  </span>
                </label>
              ))}
            </div>
            {support?.experimental && (
              <p className="note warning">{t.experimentalNote(camera.name)}</p>
            )}
          </div>
        </section>

        <section className="section" aria-labelledby="modules-label">
          <h2 id="modules-label">{t.modules}</h2>
          <div className="section-body">
            <p className="note">{t.modulesNote}</p>
            <div role="group" aria-labelledby="modules-label" className="modules">
              {MODULES.filter((module) => available.includes(module.key)).map((module) => (
                <label key={module.key} className="module">
                  <input
                    type="checkbox"
                    checked={enabled.has(module.key)}
                    onChange={() => toggleModule(module.key)}
                  />
                  <span className="module-text">
                    <span className="module-name">{t.moduleNames[module.key]}</span>
                    <span className="module-description">
                      {t.modulesText[module.key]}
                    </span>
                    {module.key === 'imageControl' &&
                      enabled.has('imageControl') &&
                      selection?.size === 0 && (
                        <span className="module-warning">{t.imageControlEmpty}</span>
                      )}
                  </span>
                </label>
              ))}
            </div>
          </div>
        </section>

        <section className="section" aria-labelledby="effects-label" ref={prefetchInView}>
          <h2 id="effects-label">{t.effects}</h2>
          {!available.includes('imageControl') ? (
            <p className="status section-body">{t.effectsUnavailable(camera.name)}</p>
          ) : plan.includeEffects ? (
            <ErrorBoundary
              resetKey={String(catalogAttempt)}
              fallback={
                <div className="section-body load-error">
                  <p className="status error">{t.loadError}</p>
                  <button
                    type="button"
                    className="button small"
                    onClick={() => setCatalogAttempt((attempt) => attempt + 1)}
                  >
                    {t.retryLoad}
                  </button>
                </div>
              }
            >
              <Suspense fallback={<p className="status section-body">{t.loading}</p>}>
                <EffectsSection
                  strings={t}
                  language={language}
                  selection={selection}
                  onSelectionChange={changeSelection}
                  imported={imported}
                  onImport={addImported}
                  onRemove={removeImported}
                />
              </Suspense>
            </ErrorBoundary>
          ) : (
            <p className="status section-body">{t.effectsOff}</p>
          )}
        </section>

        <section className="section" aria-labelledby="package-label">
          <h2 id="package-label">{t.package}</h2>
          <div className="package">
            <dl>
              <dt>{t.camera}</dt>
              <dd>{camera.name}</dd>
              <dt>{t.firmware}</dt>
              <dd>{support?.firmware ?? '—'}</dd>
              <dt>{t.effectCount}</dt>
              <dd>
                {plan.includeEffects ? (
                  <ErrorBoundary resetKey={String(catalogAttempt)} fallback="—">
                    <Suspense fallback="—">
                      <CheckedCount selection={selection} />
                    </Suspense>
                  </ErrorBoundary>
                ) : (
                  0
                )}
              </dd>
              <dt>{t.modules}</dt>
              <dd>
                {plan.visible.length > 0
                  ? MODULES.filter((module) => plan.visible.includes(module.key))
                      .map((module) => t.moduleNames[module.key])
                      .join(t.listSeparator)
                  : '—'}
              </dd>
              <dd className="package-note">{t.firmwareNote}</dd>
            </dl>
            <div className="package-action">
              <button
                type="button"
                className="download"
                disabled={!targets || plan.visible.length === 0 || build.state === 'building'}
                aria-busy={build.state === 'building'}
                onClick={download}
                onPointerEnter={prefetch}
                onFocus={prefetch}
              >
                {t.download}
              </button>
              <div aria-live="polite">
                {plan.visible.length === 0 && <p className="status error">{t.noModules}</p>}
                {build.state === 'building' && (
                  <p className="status">
                    {build.received > 0 ? t.downloading(megabytes(build.received)) : t.building}
                  </p>
                )}
                {build.state === 'ready' && <p className="status">{t.ready}</p>}
                {build.state === 'failed' && <p className="status error">{t.buildError}</p>}
              </div>
              <ul className="package-links">
                <li>
                  <a href={FEATURE_REQUEST_FORM}>{t.featureRequest}</a>
                </li>
                <li>
                  <a href={DONATE}>{t.donate}</a>
                </li>
              </ul>
            </div>
          </div>
        </section>

        <section className="section" aria-labelledby="install-label">
          <h2 id="install-label">{t.install}</h2>
          <ol className="steps">
            {t.steps.map((parts, index) => (
              <li key={index}>
                <RichText
                  parts={parts.map((part) =>
                    part === MODESET_TOKEN ? `\`${support?.modeset ?? ''}` : part,
                  )}
                />
              </li>
            ))}
          </ol>
          <p className="note">
            <RichText parts={t.updateNote} />
          </p>
        </section>

        <section className="section" aria-labelledby="remove-label">
          <h2 id="remove-label">{t.remove}</h2>
          <ol className="steps">
            {t.removeSteps.map((parts, index) => (
              <li key={index}>
                <RichText
                  parts={parts.map((part) =>
                    part === MODESET_TOKEN ? `\`${support?.modeset ?? ''}` : part,
                  )}
                />
              </li>
            ))}
          </ol>
          <div className="package-action">
            <button
              type="button"
              className="download secondary"
              disabled={!support || scriptsOff === 'building'}
              aria-busy={scriptsOff === 'building'}
              onClick={downloadScriptsOff}
            >
              {t.scriptsOff}
            </button>
            <div aria-live="polite">
              {scriptsOff === 'failed' && <p className="status error">{t.scriptsOffError}</p>}
            </div>
          </div>
        </section>
      </main>

      <footer className="footer">
        <div className="footer-links">
          <ul>
            <li>
              <a href={guideUrl('RECOVERY', language)}>{t.recovery}</a>
            </li>
            <li>
              <a href={guideUrl('DUMP', language)}>{t.firmwareDump}</a>
            </li>
          </ul>
          <ul>
            <li>
              <a href={`${REPOSITORY}/blob/main/effects/NOTICE.md`}>{t.licenses}</a>
            </li>
            <li>
              <a href={siteUrl('THIRD_PARTY_LICENSES.txt')}>{t.thirdParty}</a>
            </li>
          </ul>
        </div>
        <div className="footer-meta">
          <p>{t.trademarks}</p>
          <a href={`${REPOSITORY}/releases/tag/v${__GRKIT_VERSION__}`}>GRKit {__GRKIT_VERSION__}</a>
        </div>
      </footer>
    </div>
  );
}

/** The number of checked effects. Before the first change, it needs the catalog. */
function CheckedCount({ selection }: { selection: Selection }) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  return resolveSelection(
    selection,
    catalog.effects.map((effect) => effect.id),
  ).size;
}
