import { useRef } from 'react'
import { gsap, useGSAP } from '../lib/gsap'
import { reducedMotion } from './SmoothScroll'

// The logo's wave: six half-swings that die out into a flat line, then the caret.
const W = 300
const H = 120
const MID = H / 2
const SWINGS = [1, 0.83, 0.61, 0.39, 0.19, 0.07]
const SEG = 40

function wavePath(energy: number, t: number) {
  let d = `M 6 ${MID}`
  SWINGS.forEach((amp, i) => {
    const x0 = 6 + i * SEG
    // While listening each swing breathes on its own; at rest it keeps the logo's shape.
    const jitter = 1 + 0.35 * Math.sin(t * 7.3 + i * 1.9) * energy
    const a = amp * energy * jitter * (MID - 8) * 2
    const sign = i % 2 === 0 ? -1 : 1
    d += ` Q ${x0 + SEG / 2} ${MID + sign * a} ${x0 + SEG} ${MID}`
  })
  return d + ` L ${W - 6} ${MID}`
}

export function WaveToText({ sentences }: { sentences: string[] }) {
  const root = useRef<HTMLDivElement>(null)
  const path = useRef<SVGPathElement>(null)
  const text = useRef<HTMLSpanElement>(null)

  useGSAP(
    () => {
      const p = path.current!
      const out = text.current!

      if (reducedMotion()) {
        p.setAttribute('d', wavePath(0.55, 0))
        out.textContent = sentences[0]
        return
      }

      const state = { energy: 0, t: 0 }
      const draw = () => {
        state.t += gsap.ticker.deltaRatio() / 60
        p.setAttribute('d', wavePath(state.energy, state.t))
      }
      gsap.ticker.add(draw)

      const tl = gsap.timeline({ repeat: -1, delay: 0.9 })
      gsap.from(p, { drawSVG: '0%', duration: 1.1, ease: 'power2.inOut', delay: 0.3 })

      sentences.forEach((sentence) => {
        const typed = { n: 0 }
        tl.to(state, { energy: 1, duration: 0.5, ease: 'power2.out' })
          .to(state, { energy: 0.85, duration: 1.4, ease: 'none' })
          .to(state, { energy: 0, duration: 0.7, ease: 'power3.in' })
          .to(typed, {
            n: sentence.length,
            duration: sentence.length * 0.035,
            ease: 'none',
            onUpdate: () => {
              out.textContent = sentence.slice(0, Math.round(typed.n))
            },
          })
          .to({}, { duration: 2.4 })
          .to(out, { autoAlpha: 0, duration: 0.35 })
          .call(() => {
            out.textContent = ''
            gsap.set(out, { autoAlpha: 1 })
          })
      })

      return () => gsap.ticker.remove(draw)
    },
    { scope: root, dependencies: [sentences] },
  )

  return (
    <div ref={root} className="flex flex-col gap-3 md:flex-row md:items-center md:gap-6" aria-live="off">
      <svg
        viewBox={`0 0 ${W} ${H}`}
        className="h-16 w-40 shrink-0 md:h-24 md:w-60"
        aria-hidden="true"
      >
        <path
          ref={path}
          d={wavePath(0.55, 0)}
          fill="none"
          stroke="currentColor"
          strokeWidth="7"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <p className="min-h-[3.2em] max-w-[22ch] text-2xl leading-snug italic md:min-h-[2.6em] md:text-3xl">
        <span ref={text} />
        <span className="caret" aria-hidden="true" />
      </p>
    </div>
  )
}
