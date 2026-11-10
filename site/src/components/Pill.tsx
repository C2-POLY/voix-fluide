import { useEffect, useState } from 'react'

export type PillState = 'listen' | 'transcribe' | 'pasted'

// The app's pill (draw_pill in main.rs, Medium size), redrawn in HTML: 8 bars 3.5 pt wide
// and 4.5 pt apart, 6 to 28 pt tall, on a black panel with an 18 pt radius.
const BARS = [0.45, 0.75, 1, 0.6, 0.9, 0.5, 0.8, 0.4]
const RED = '#ff453a'
const BLUE = '#1a75ff'
const LIGHT_BLUE = '#9cc2ff'
const GREEN_TEXT = '#5bd98a'
const MUTED = '#8c92a0'

export function Pill({ state, label, text }: { state: PillState; label: string; text?: string }) {
  const [secs, setSecs] = useState(3)

  useEffect(() => {
    if (state !== 'listen') return
    setSecs(3)
    const id = window.setInterval(() => setSecs((s) => s + 1), 1000)
    return () => window.clearInterval(id)
  }, [state])

  const transcribing = state === 'transcribe'

  return (
    <div
      className={`@container max-w-full rounded-[18px] border px-[18px] py-3 font-sans text-[13px] shadow-[0_24px_60px_-20px_rgba(0,0,0,0.9)] transition-[width,border-color] duration-500 motion-reduce:transition-none ${transcribing ? 'w-[230px] border-[#1a3a6e]' : 'w-[340px] border-[#2e3546]'}`}
      style={{ background: 'rgba(0, 0, 0, 0.92)' }}
      role="img"
      aria-label={text && state === 'pasted' ? `${label} : ${text}` : label}
    >
      {state === 'listen' && (
        <div className="flex h-8 items-center">
          {/* Like the app, the word only shows when there is room beside the waveform. */}
          <span className="flex shrink-0 items-center gap-2.5 @[290px]:w-24">
            <span className="pulse relative h-2 w-2 rounded-full" style={{ background: RED, color: RED }} />
            <span className="hidden text-[12px] @[290px]:inline" style={{ color: MUTED }}>
              {label}
            </span>
          </span>
          <span className="flex min-w-0 flex-1 items-center justify-center gap-[4.5px]">
            {BARS.map((h, i) => (
              <span
                key={i}
                className="bar w-[3.5px] rounded-full bg-white"
                style={{ height: `${6 + h * 22}px`, animationDelay: `${(i * 0.13) % 0.9}s`, animationDuration: '0.55s' }}
              />
            ))}
            {/* The logo's ending: a flat dash, then the blue caret. */}
            <span className="ml-[2px] h-[3.5px] w-[9px] rounded-full bg-white" />
            <span className="caret-blink h-[17px] w-[2px] rounded-[1px]" style={{ background: BLUE }} />
          </span>
          <span className="shrink-0 pl-2 text-right font-mono text-[12px] @[290px]:w-12" style={{ color: MUTED }}>
            {`0:${String(secs % 60).padStart(2, '0')}`}
          </span>
        </div>
      )}

      {transcribing && (
        <div className="flex h-8 items-center justify-center gap-[10px]">
          <span className="flex items-center gap-[4.5px]">
            {BARS.map((_, i) => (
              <span
                key={i}
                className="travel h-[18px] w-[3.5px] rounded-full"
                style={{ background: BLUE, animationDelay: `${i * 0.12}s` }}
              />
            ))}
          </span>
          <span className="whitespace-nowrap" style={{ color: LIGHT_BLUE }}>
            {label}
          </span>
        </div>
      )}

      {state === 'pasted' && (
        <>
          <div className="flex h-8 items-center gap-[7px]">
            <svg viewBox="0 0 14 14" className="h-[14px] w-[14px] shrink-0" aria-hidden="true">
              <circle cx="7" cy="7" r="7" fill="#0f2a1a" />
              <path d="M3.8 7.2 6 9.5l4.2-4.9" fill="none" stroke={GREEN_TEXT} strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="text-[11px]" style={{ color: GREEN_TEXT }}>
              {label}
            </span>
          </div>
          {text && <p className="mt-1 mb-1 leading-snug text-[#eceef2]">{text}</p>}
        </>
      )}
    </div>
  )
}
