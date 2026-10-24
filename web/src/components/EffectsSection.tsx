import {
  Suspense,
  use,
  useCallback,
  useRef,
  useState,
  useTransition,
  type ChangeEvent,
} from 'react';
import { Handle, HandleRoot, Item, Provider, Root } from 'react-compare-slider/components';
import { useReactCompareSlider } from 'react-compare-slider/hooks';
import { PREVIEW_TARGET } from '../cameras';
import {
  effectUrl,
  loadCatalog,
  MAX_EFFECTS,
  previewTable,
  type Effect,
  type ImportedEffect,
} from '../catalog';
import type { Language, Strings } from '../i18n';
import { forget, preview, type PreviewEffect, type Source } from '../preview/render';
import { DEFAULT_SAMPLE } from '../preview/samples';
import { PhotoLoader } from '../preview/raw';
import { resolveSelection, selectImport, type Selection } from '../selection';
import { STOCKS } from '../stocks';
import { EffectIcon } from './EffectIcon';
import { ErrorBoundary } from './ErrorBoundary';
import { ImportDialog } from './ImportDialog';

const DEFAULT_PREVIEW = 'portra-400-endura-v1';

/** The film number, then the print film of a cine look: "5219 · 2393 print". */
function stockText(stock: (typeof STOCKS)[string] & {}, t: Strings): string {
  return stock.print ? `${stock.film} · ${t.printFilm(stock.print)}` : stock.film;
}

interface Props {
  strings: Strings;
  language: Language;
  selection: Selection;
  onSelectionChange: (selection: ReadonlySet<string>) => void;
  /** The effects that this page fitted from LUTs, after the catalog effects. */
  imported: readonly ImportedEffect[];
  onImport: (effect: ImportedEffect) => void;
  onRemove: (effect: ImportedEffect) => void;
}

export function EffectsSection({
  strings: t,
  selection,
  onSelectionChange,
  imported,
  onImport,
  onRemove,
}: Props) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  const effects: readonly Effect[] = [...catalog.effects, ...imported];
  const checked = resolveSelection(
    selection,
    catalog.effects.map((effect) => effect.id),
  );
  const taken = new Set(effects.map((effect) => effect.id));
  const [previewId, setPreviewId] = useState(DEFAULT_PREVIEW);
  const [source, setSource] = useState<Source>(DEFAULT_SAMPLE);
  const photo = useRef<Source | null>(null);
  const [loader] = useState(() => new PhotoLoader());
  const [decoding, setDecoding] = useState(false);
  const ownPhoto = useCallback(
    () => () => {
      loader.cancel();
      if (photo.current) forget(photo.current);
      photo.current = null;
    },
    [loader],
  );
  const [photoError, setPhotoError] = useState(false);
  const [previewAttempt, setPreviewAttempt] = useState(0);
  const [isPending, startTransition] = useTransition();
  const photoCount = useRef(0);

  const shown = effects.find((effect) => effect.id === previewId) ?? effects[0];
  const name = shown?.name ?? '';

  function toggle(effect: Effect) {
    const next = new Set(checked);
    if (next.has(effect.id)) {
      next.delete(effect.id);
    } else if (next.size < MAX_EFFECTS) {
      next.add(effect.id);
    }
    onSelectionChange(next);
  }

  function showEffect(effect: Effect) {
    startTransition(() => setPreviewId(effect.id));
  }

  /** Adds an import and checks it if it fits closely and the package has room. Returns true
   * when the package had no room. */
  function importEffect(effect: ImportedEffect): boolean {
    onImport(effect);
    const { selection: next, full } = selectImport(checked, effect.id, effect.imported.grade);
    if (next !== checked) onSelectionChange(next);
    showEffect(effect);
    return full;
  }

  function showSample() {
    loader.cancel();
    setDecoding(false);
    setPhotoError(false);
    startTransition(() => setSource(DEFAULT_SAMPLE));
    if (photo.current) forget(photo.current);
    photo.current = null;
  }

  async function openPhoto(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) return;
    setPhotoError(false);
    setDecoding(true);
    try {
      const image = await loader.open(file);
      // A replacement, sample selection, or unmount cancelled this request.
      if (!image) return;
      const next: Source = { key: `photo-${++photoCount.current}`, load: async () => image };
      const previous = photo.current;
      photo.current = next;
      startTransition(() => setSource(next));
      if (previous) forget(previous);
      setDecoding(false);
    } catch {
      setPhotoError(true);
      setDecoding(false);
    }
  }

  const sourceLabel = source === DEFAULT_SAMPLE ? t.samplePhoto : t.yourPhoto;

  function retryPreview() {
    forget(source);
    setPreviewAttempt((attempt) => attempt + 1);
  }

  return (
    <div className="section-body" ref={ownPhoto}>
      <div className="preview-row">
        <div className="viewer">
          {shown && (
            <ErrorBoundary
              resetKey={`${source.key}\n${shown.id}\n${previewAttempt}`}
              fallback={
                <div className="frame">
                  <div className="frame-placeholder">
                    <p>{t.previewError}</p>
                    <button type="button" className="button small" onClick={retryPreview}>
                      {t.retryPreview}
                    </button>
                  </div>
                </div>
              }
            >
              <Suspense fallback={<Placeholder text={sourceLabel} busy />}>
                <PreviewFrame
                  source={source}
                  effect={{
                    id: shown.id,
                    table: previewTable(catalog.target, shown),
                    size: catalog.lutSize,
                    baseline: effectUrl(catalog.target, catalog.baseline),
                  }}
                  compareLabel={t.compare}
                  busy={isPending || decoding}
                  originalLabel={t.original}
                  originalAlt={sourceLabel}
                  effectName={name}
                />
              </Suspense>
            </ErrorBoundary>
          )}
        </div>

        <div className="preview-side">
          <div className="field">
            <span className="field-label">{t.previewing}</span>
            <span className="previewing">{name}</span>
          </div>
          <label htmlFor="photo-input" className="upload">
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M12 16V4" />
              <path d="M7 9l5-5 5 5" />
              <path d="M4 20h16" />
            </svg>
            <span>{t.upload}</span>
          </label>
          <input
            id="photo-input"
            className="visually-hidden"
            type="file"
            accept=".dng,image/x-adobe-dng"
            onChange={openPhoto}
          />
          {decoding && (
            <p role="status" className="status">
              {t.decodingPhoto}
            </p>
          )}
          {photoError && (
            <p role="alert" className="status error">
              {t.photoError}
            </p>
          )}
          {source !== DEFAULT_SAMPLE && (
            <button type="button" className="button small" onClick={showSample}>
              {t.useSample}
            </button>
          )}
          <p className="note">{t.uploadNote}</p>
          <p className="note">{t.previewNote}</p>
          {source === DEFAULT_SAMPLE && (
            <p className="note">
              <a
                href="https://www.flickr.com/photos/cafetricot/51803682755/"
                target="_blank"
                rel="noreferrer"
              >
                {t.sampleCredit}
              </a>
            </p>
          )}
          {catalog.baseGamma && (
            <ImportDialog
              strings={t}
              baseGamma={catalog.baseGamma}
              taken={taken}
              onImport={importEffect}
            />
          )}
        </div>
      </div>

      <div className="toolbar">
        <p>
          {t.count(MAX_EFFECTS)[0]}
          <span className="count">{checked.size}</span>
          {t.count(MAX_EFFECTS)[1]}
        </p>
        <div>
          <button
            type="button"
            className="button"
            onClick={() =>
              onSelectionChange(new Set(effects.slice(0, MAX_EFFECTS).map((effect) => effect.id)))
            }
          >
            {t.selectAll}
          </button>
          <button type="button" className="button" onClick={() => onSelectionChange(new Set())}>
            {t.clear}
          </button>
        </div>
      </div>
      <p className="note">{t.libraryNote}</p>

      <div className="table-box">
        <table>
          <thead>
            <tr>
              <th scope="col" style={{ width: 44 }}>
                <span className="visually-hidden">{t.include}</span>
              </th>
              <th scope="col" style={{ width: 64 }}>
                {t.icon}
              </th>
              <th scope="col">{t.effect}</th>
              <th scope="col">{t.look}</th>
              <th scope="col">{t.type}</th>
              <th scope="col" style={{ width: 88 }}>
                <span className="visually-hidden">{t.preview}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {effects.map((effect) => (
              <tr key={effect.id}>
                <td>
                  <input
                    id={`effect-${effect.id}`}
                    type="checkbox"
                    checked={checked.has(effect.id)}
                    disabled={!checked.has(effect.id) && checked.size >= MAX_EFFECTS}
                    onChange={() => toggle(effect)}
                  />
                </td>
                <td>
                  <EffectIcon effect={effect} />
                </td>
                <td>
                  <label htmlFor={`effect-${effect.id}`} className="effect-name">
                    {effect.name}
                  </label>
                  {STOCKS[effect.id] && (
                    <span className="effect-stock">{stockText(STOCKS[effect.id]!, t)}</span>
                  )}
                </td>
                <td>
                  {effect.imported ? (
                    <>
                      <span className="effect-source">{effect.imported.source}</span>
                      <span className="effect-stock">{t.fitText[effect.imported.grade]}</span>
                    </>
                  ) : (
                    (t.looks[effect.id] ?? effect.look)
                  )}
                </td>
                <td>
                  {effect.monochrome ? t.mono : t.color}
                  {effect.imported && <span className="effect-stock">{t.imported}</span>}
                </td>
                <td>
                  <div className="row-actions">
                    <button
                      type="button"
                      className="button small"
                      aria-pressed={effect.id === shown?.id}
                      aria-label={t.previewOf(effect.name)}
                      onClick={() => showEffect(effect)}
                    >
                      {t.preview}
                    </button>
                    {effect.imported && (
                      <button
                        type="button"
                        className="button small"
                        aria-label={t.removeOf(effect.name)}
                        onClick={() => onRemove(effect)}
                      >
                        {t.removeEffect}
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

interface FrameProps {
  source: Source;
  effect: PreviewEffect;
  compareLabel: string;
  busy: boolean;
  originalLabel: string;
  originalAlt: string;
  effectName: string;
}

/** The baseline development on the left and the effect on the right. */
function PreviewFrame({
  source,
  effect,
  compareLabel,
  busy,
  originalLabel,
  originalAlt,
  effectName,
}: FrameProps) {
  const result = use(preview(source, effect));
  const comparison = useReactCompareSlider({ clip: 'itemTwo' });
  return (
    <div className="frame" aria-busy={busy}>
      <Provider {...comparison}>
        <Root className="image-comparison">
          <Item item="itemOne">
            <img src={result.original} alt={`${originalAlt} — ${originalLabel}`} />
          </Item>
          <Item item="itemTwo" className="comparison-effect">
            <img src={result.processed} alt={`${originalAlt} — ${effectName}`} />
          </Item>
          <HandleRoot aria-label={compareLabel}>
            <Handle buttonStyle={{ width: 44, height: 44 }} />
          </HandleRoot>
        </Root>
      </Provider>
      <span className="tag">{effectName}</span>
    </div>
  );
}

function Placeholder({ text, busy = false }: { text: string; busy?: boolean }) {
  return (
    <div className="frame" aria-busy={busy}>
      <span className="frame-placeholder">{text}</span>
    </div>
  );
}
