import { useEffect, useState } from 'react'

export type PillState = 'listen' | 'transcribe' | 'pasted'

const BARS = [0.45, 0.75, 1, 0.6, 0.9, 0.5, 0.8, 0.4]

// A faithful copy of the app's floating pill (black panel, white level bars), drawn in HTML.
export function Pill({ state, label, text }: { state: PillState; label: string; text?: string }) {
  const [secs, setSecs] = useState(3)

  useEffect(() => {
    if (state !== 'listen') return
    setSecs(3)
    const id = window.setInterval(() => setSecs((s) => s + 1), 1000)
    return () => window.clearInterval(id)
  }, [state])

  const dot = state === 'listen' ? '#ff453a' : state === 'transcribe' ? '#1a75ff' : '#30d158'

  return (
    <div
      className="w-full max-w-[340px] rounded-[18px] border border-[#2a2f3c] bg-black px-[18px] py-3 font-sans text-[13px] text-[#c9ccd3] shadow-[0_24px_60px_-20px_rgba(22,19,15,0.55)]"
      role="img"
      aria-label={text ? `${label} : ${text}` : label}
    >
      <div className="flex h-8 items-center gap-2">
        <span className="h-[7px] w-[7px] shrink-0 rounded-full" style={{ background: dot }} />
        <span className="w-24 shrink-0 text-[12px] text-[#9a9fab]">{label}</span>
        <div className="flex flex-1 items-center justify-center gap-[4.5px]">
          {state === 'pasted' ? (
            <span className="dots flex gap-[5px]">
              {BARS.map((_, i) => (
                <span key={i} className="h-[5px] w-[5px] rounded-full bg-[#30d158]" style={{ animationDelay: `${i * 0.08}s` }} />
              ))}
            </span>
          ) : (
            BARS.map((h, i) => (
              <span
                key={i}
                className="bar w-[3.5px] rounded-full"
                style={{
                  height: `${h * 28}px`,
                  background: state === 'listen' ? '#f2f4f8' : '#1a75ff',
                  animationDelay: `${(i * 0.13) % 0.9}s`,
                  animationDuration: state === 'listen' ? '0.55s' : '0.9s',
                }}
              />
            ))
          )}
        </div>
        <span className="w-10 shrink-0 text-right font-mono text-[12px] text-[#9a9fab]">
          {state === 'listen' ? `0:${String(secs % 60).padStart(2, '0')}` : ''}
        </span>
      </div>
      {state === 'pasted' && text && <p className="mt-1 mb-1 pl-[15px] leading-snug text-[#e6e8ec]">{text}</p>}
    </div>
  )
}
