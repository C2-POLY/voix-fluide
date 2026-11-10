import { useEffect, useRef, useState, type ReactNode } from 'react'
import { copy, type Copy, type Lang } from './i18n'
import { gsap, ScrollTrigger, SplitText, useGSAP } from './lib/gsap'
import { SmoothScroll } from './components/SmoothScroll'
import { WaveToText } from './components/WaveToText'
import { Pill, type PillState } from './components/Pill'

const REPO = 'https://github.com/C2-POLY/voix-fluide'
const DMG = `${REPO}/releases/latest/download/Voix-Fluide.dmg`
const INSTALL = ['git clone https://github.com/C2-POLY/voix-fluide.git', 'cd voix-fluide', 'cargo run']

function initialLang(): Lang {
  const fromUrl = new URLSearchParams(window.location.search).get('lang')
  if (fromUrl === 'fr' || fromUrl === 'en') return fromUrl
  try {
    const saved = window.localStorage.getItem('vf-lang')
    if (saved === 'fr' || saved === 'en') return saved
  } catch {
    // Storage can be blocked; French is the default.
  }
  return 'fr'
}

export function App() {
  const [lang, setLang] = useState<Lang>(initialLang)

  useEffect(() => {
    document.documentElement.lang = lang
    document.title = copy[lang].htmlTitle
    try {
      window.localStorage.setItem('vf-lang', lang)
    } catch {
      // Not remembered, which is fine.
    }
  }, [lang])

  return (
    <SmoothScroll>
      <Page key={lang} c={copy[lang]} lang={lang} setLang={setLang} />
    </SmoothScroll>
  )
}

function Page({ c, lang, setLang }: { c: Copy; lang: Lang; setLang: (l: Lang) => void }) {
  const main = useRef<HTMLDivElement>(null)

  useGSAP(
    () => {
      const mm = gsap.matchMedia()

      mm.add('(prefers-reduced-motion: no-preference)', () => {
        // Hero: each line rises out of its own mask.
        gsap.from('.hero-line > span', {
          yPercent: 112,
          duration: 1.3,
          stagger: 0.11,
          ease: 'expo.out',
          delay: 0.1,
        })
        gsap.from('[data-hero-fade]', { autoAlpha: 0, y: 18, duration: 1, stagger: 0.1, delay: 0.7, ease: 'power3.out' })
        gsap.to('.hero-title', {
          yPercent: -12,
          ease: 'none',
          scrollTrigger: { trigger: '.hero-title', start: 'top top', end: 'bottom top', scrub: true },
        })

        // Section titles: line by line, when they come into view.
        gsap.utils.toArray<HTMLElement>('.split-title').forEach((el) => {
          SplitText.create(el, {
            type: 'lines',
            mask: 'lines',
            autoSplit: true,
            onSplit: (self) =>
              gsap.from(self.lines, {
                yPercent: 105,
                duration: 1.1,
                stagger: 0.09,
                ease: 'expo.out',
                scrollTrigger: { trigger: el, start: 'clamp(top 85%)', once: true },
              }),
          })
        })

        gsap.set('[data-reveal]', { autoAlpha: 0, y: 28 })
        ScrollTrigger.batch('[data-reveal]', {
          start: 'clamp(top 88%)',
          once: true,
          onEnter: (batch) => gsap.to(batch, { autoAlpha: 1, y: 0, duration: 0.9, stagger: 0.08, ease: 'power3.out' }),
        })

        gsap.utils.toArray<HTMLElement>('[data-rule]').forEach((el) => {
          gsap.from(el, {
            scaleX: 0,
            transformOrigin: 'left center',
            duration: 1.4,
            ease: 'expo.inOut',
            scrollTrigger: { trigger: el, start: 'clamp(top 92%)', once: true },
          })
        })

        // Plates are pasted onto the page: they settle as they scroll in.
        gsap.utils.toArray<HTMLElement>('[data-plate]').forEach((el) => {
          gsap.fromTo(
            el,
            { rotate: -2.5, scale: 0.94, y: 40 },
            { rotate: 0, scale: 1, y: 0, ease: 'none', scrollTrigger: { trigger: el, start: 'top bottom', end: 'center 55%', scrub: 0.6 } },
          )
        })

        // The question is spoken word by word, then the answer is printed.
        const qa = gsap.timeline({ scrollTrigger: { trigger: '#qa', start: 'clamp(top 75%)', once: true } })
        const words = SplitText.create('#question', { type: 'words' }).words
        qa.from(words, { autoAlpha: 0, y: 8, duration: 0.5, stagger: 0.06, ease: 'power2.out' })
          .from('#answer', { autoAlpha: 0, x: -14, duration: 0.8, ease: 'power3.out' }, '+=0.5')
          .from('#spoken', { autoAlpha: 0, duration: 0.6 }, '-=0.2')
      })

      document.fonts?.ready.then(() => ScrollTrigger.refresh())
    },
    { scope: main },
  )

  return (
    <div ref={main} className="mx-auto max-w-[1240px] px-4 md:px-10">
      <Masthead c={c} lang={lang} setLang={setLang} />
      <Hero c={c} />
      <Contents c={c} />
      <How c={c} />
      <Local c={c} />
      <PillSection c={c} />
      <ClaudeSection c={c} />
      <TryIt c={c} />
      <Colophon c={c} />
    </div>
  )
}

function Masthead({ c, lang, setLang }: { c: Copy; lang: Lang; setLang: (l: Lang) => void }) {
  return (
    <header className="pt-5">
      <div className="flex items-center justify-between gap-4">
        <a href="#top" className="flex items-center gap-2.5" aria-label="Voix Fluide">
          <LogoMark className="h-5 w-auto" />
          <span className="smallcaps font-medium">Voix Fluide</span>
        </a>
        <p className="smallcaps hidden text-mist md:block">
          {c.masthead.issue} · {c.masthead.date} · {c.masthead.tag}
        </p>
        <div className="smallcaps flex items-center gap-1" role="group" aria-label="Langue / Language">
          {(['fr', 'en'] as const).map((l, i) => (
            <span key={l} className="flex items-center gap-1">
              {i > 0 && <span className="text-mist">/</span>}
              <button
                type="button"
                onClick={() => setLang(l)}
                aria-pressed={lang === l}
                className={`cursor-pointer px-1 py-1 ${lang === l ? 'text-snow underline decoration-signal decoration-2 underline-offset-4' : 'text-mist hover:text-snow'}`}
              >
                {l.toUpperCase()}
              </button>
            </span>
          ))}
        </div>
      </div>
      <p className="smallcaps mt-3 text-mist md:hidden">
        {c.masthead.issue} · {c.masthead.date}
      </p>
      <div className="mt-4 border-t-[3px] border-snow" />
      <div className="mt-[3px] border-t border-snow" />
    </header>
  )
}

function Hero({ c }: { c: Copy }) {
  const [l1, l2, l3] = c.hero.lines
  return (
    <section id="top" className="pt-10 pb-16 md:pt-16 md:pb-24">
      <h1 className="hero-title display text-[clamp(3.6rem,15.5vw,13.5rem)] leading-[0.84] font-[340]">
        <span className="hero-line line-mask block">
          <span className="block">{l1}</span>
        </span>
        <span className="hero-line line-mask block md:pl-[16%]">
          <span className="block">{l2}</span>
        </span>
        <span className="hero-line line-mask block md:pl-[8%]">
          <span className="block italic">
            {l3.replace(/\.$/, '')}
            <span className="text-signal">.</span>
          </span>
        </span>
      </h1>

      <div className="mt-12 grid gap-10 md:mt-16 md:grid-cols-12 md:gap-8">
        <div data-hero-fade className="md:col-span-4">
          <p className="text-lg leading-relaxed md:text-xl">
            <span className="float-left mt-1 mr-2 font-serif text-[3.6em] leading-[0.8] text-signal italic">
              {c.hero.lede.charAt(0)}
            </span>
            {c.hero.lede.slice(1)}
            <span className="mt-6 block space-y-2 text-base text-mist">
              {c.hero.keys.map((k) => (
                <span key={k} className="block">
                  {rich(k)}
                </span>
              ))}
            </span>
          </p>
          <Download c={c} />
        </div>
        <div data-hero-fade className="md:col-span-7 md:col-start-6">
          <WaveToText sentences={c.hero.sentences} />
        </div>
      </div>
      <p data-hero-fade className="smallcaps mt-14 text-mist">
        {c.hero.cue} <span aria-hidden="true">↓</span>
      </p>
    </section>
  )
}

const SECTIONS = ['how', 'local', 'pill', 'claude', 'try'] as const
const ROMAN = ['I', 'II', 'III', 'IV', 'V']

function Contents({ c }: { c: Copy }) {
  return (
    <nav aria-label="Sommaire" className="border-y border-snow py-3">
      <ol className="smallcaps flex flex-wrap gap-x-8 gap-y-2">
        {SECTIONS.map((id, i) => (
          <li key={id}>
            <a href={`#${id}`} className="group inline-flex gap-2 py-1 hover:text-signal">
              <span className="text-signal">{ROMAN[i]}</span>
              <span>{c.nav[id]}</span>
            </a>
          </li>
        ))}
      </ol>
    </nav>
  )
}

function Section({ id, kicker, title, children }: { id: string; kicker: string; title: string; children: ReactNode }) {
  return (
    <section id={id} className="grid scroll-mt-6 gap-8 py-20 md:grid-cols-12 md:gap-8 md:py-32">
      <div className="min-w-0 md:col-span-3">
        <p className="smallcaps text-signal md:sticky md:top-8">{kicker}</p>
      </div>
      <div className="min-w-0 md:col-span-9">
        <h2 className="split-title display text-[clamp(2.6rem,7vw,6.2rem)] leading-[0.95] font-[360]">{title}</h2>
        {children}
      </div>
    </section>
  )
}

function Rule() {
  return <div data-rule className="h-px bg-snow" />
}

function How({ c }: { c: Copy }) {
  return (
    <>
      <Section id="how" kicker={c.how.kicker} title={c.how.title}>
        <ol className="mt-14 grid gap-10 md:grid-cols-3 md:gap-0 md:divide-x md:divide-snow/25">
          {c.how.steps.map((s, i) => (
            <li key={s.word} data-reveal className="md:px-7 md:first:pl-0 md:last:pr-0">
              <span className="display block text-6xl text-signal italic">{i + 1}</span>
              <h3 className="display mt-3 text-3xl font-[420] md:text-4xl">{s.word}</h3>
              <p className="mt-4 leading-relaxed text-mist">{rich(s.body)}</p>
            </li>
          ))}
        </ol>
        <Shot src="accueil.jpg" width={1600} height={1066} caption={c.how.figure} />
        <p data-reveal className="mt-10 flex max-w-[60ch] gap-4 leading-relaxed text-mist">
          <MenuBarIcon />
          <span>{c.how.menubar}</span>
        </p>
      </Section>
      <Rule />
    </>
  )
}

function Local({ c }: { c: Copy }) {
  return (
    <>
      <Section id="local" kicker={c.local.kicker} title={c.local.title}>
        <p data-reveal className="mt-8 max-w-[52ch] text-xl leading-relaxed md:text-2xl">
          {c.local.body}
        </p>
        <dl className="mt-14 border-t border-snow">
          {c.local.rows.map(([k, v]) => (
            <div key={k} data-reveal className="flex flex-col gap-1 border-b border-snow/25 py-4 md:flex-row md:items-baseline md:gap-4">
              <dt className="flex items-baseline gap-4 md:w-[45%]">
                <span className="display text-2xl font-[420]">{k}</span>
                <span className="leader hidden md:block" aria-hidden="true" />
              </dt>
              <dd className="font-mono text-[0.85rem] leading-relaxed text-mist md:flex-1">{v}</dd>
            </div>
          ))}
        </dl>
      </Section>
      <Rule />
    </>
  )
}

const PILL_STATES: PillState[] = ['listen', 'transcribe', 'pasted']

function PillSection({ c }: { c: Copy }) {
  const [stage, setStage] = useState(0)
  const ref = useRef<HTMLDivElement>(null)

  useGSAP(
    () => {
      // One trigger over the whole list: a fast scroll can skip a caption, never the stage.
      ScrollTrigger.create({
        trigger: '[data-stages]',
        start: 'top 60%',
        end: 'bottom 60%',
        onUpdate: (self) => setStage(Math.min(PILL_STATES.length - 1, Math.floor(self.progress * PILL_STATES.length))),
      })
    },
    { scope: ref },
  )

  const st = c.pill.states[stage]
  return (
    <>
      <Section id="pill" kicker={c.pill.kicker} title={c.pill.title}>
        <div ref={ref} className="mt-14 grid gap-10 md:grid-cols-9 md:gap-8">
          <div className="sticky top-4 z-10 md:order-2 md:col-span-5 md:top-24 md:self-start">
            <figure>
              <div className="relative flex aspect-[4/3] items-end justify-center overflow-hidden rounded-xl border border-line bg-[linear-gradient(180deg,#05070d,#0c1a33)] px-5 pb-[12%] shadow-[0_40px_90px_-40px_rgba(0,0,0,0.9)]">
                <div className="absolute inset-x-0 top-0 flex h-6 items-center gap-1.5 bg-white/5 px-3" aria-hidden="true">
                  <span className="h-2 w-2 rounded-full bg-white/15" />
                  <span className="h-2 w-2 rounded-full bg-white/15" />
                  <span className="h-2 w-2 rounded-full bg-white/15" />
                </div>
                <Pill state={PILL_STATES[stage]} label={st.label} text={c.pill.pasted} />
              </div>
              <figcaption className="smallcaps mt-3 text-mist">{c.pill.figure}</figcaption>
            </figure>
          </div>
          <ol data-stages className="md:order-1 md:col-span-4">
            {c.pill.states.map((s, i) => (
              <li
                key={s.label}
                className={`flex min-h-[38vh] flex-col justify-center border-l-2 py-6 pl-5 transition-colors duration-500 md:min-h-[55vh] ${i === stage ? 'border-signal text-snow' : 'border-snow/15 text-snow/35'}`}
              >
                <span className="smallcaps">{String(i + 1).padStart(2, '0')}</span>
                <h3 className="display mt-2 text-3xl font-[420] md:text-4xl">{s.label}</h3>
                <p className="mt-3 max-w-[34ch] leading-relaxed">{s.caption}</p>
              </li>
            ))}
          </ol>
        </div>
      </Section>
      <Rule />
    </>
  )
}

function ClaudeSection({ c }: { c: Copy }) {
  return (
    <>
      <Section id="claude" kicker={c.claude.kicker} title={c.claude.title}>
        <p data-reveal className="mt-8 max-w-[52ch] text-xl leading-relaxed md:text-2xl">
          {rich(c.claude.body)}
        </p>

        <div className="mt-16 grid gap-14 lg:grid-cols-9 lg:gap-10">
          <div className="lg:col-span-5">
            <div id="qa">
              <p className="smallcaps text-mist">{c.claude.qLabel}</p>
              <blockquote id="question" className="display mt-3 text-3xl leading-tight font-[340] italic md:text-[2.6rem]">
                {c.claude.question}
              </blockquote>
              <div id="answer" className="mt-10 border-l-2 border-signal pl-5">
                <p className="smallcaps text-mist">{c.claude.aLabel}</p>
                <p className="mt-3 text-xl leading-relaxed">{c.claude.answer}</p>
                <p id="spoken" className="smallcaps mt-4 inline-flex items-center gap-2 text-signal">
                  <SpeakerIcon /> {c.claude.spoken}
                </p>
              </div>
            </div>
          </div>

          <div className="lg:col-span-4 lg:pt-8">
            <ul className="space-y-4">
              {c.claude.points.map((p, i) => (
                <li key={p} data-reveal className="flex gap-4 leading-relaxed">
                  <span className="smallcaps pt-1 text-signal">{String.fromCharCode(97 + i)}.</span>
                  <span>{p}</span>
                </li>
              ))}
            </ul>
            <p data-reveal className="mt-10 border-y border-snow/25 py-4 text-mist italic">
              {c.claude.note}
            </p>
          </div>
        </div>
        <Shot src="claude.jpg" width={1600} height={683} caption={c.claude.figure} />
      </Section>
      <Rule />
    </>
  )
}

function Download({ c }: { c: Copy }) {
  return (
    <div className="mt-8">
      <a
        href={DMG}
        className="inline-flex items-center gap-3 rounded-lg bg-signal px-5 py-3 font-mono text-[0.95rem] text-snow transition-colors hover:bg-snow hover:text-night motion-reduce:transition-none"
      >
        <svg aria-hidden="true" viewBox="0 0 16 16" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
          <path d="M8 2v9M4 7.5 8 11.5 12 7.5M3 14h10" />
        </svg>
        {c.download.label}
      </a>
      <p className="mt-3 font-mono text-sm text-mist">{c.download.meta}</p>
      <p className="mt-2 max-w-[40ch] text-sm leading-relaxed text-mist">{c.download.firstOpen}</p>
    </div>
  )
}

function TryIt({ c }: { c: Copy }) {
  const [copied, setCopied] = useState(false)
  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(INSTALL.join('\n'))
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1800)
    } catch {
      // The commands stay selectable by hand.
    }
  }

  return (
    <Section id="try" kicker={c.tryIt.kicker} title={c.tryIt.title}>
      <p data-reveal className="mt-8 max-w-[52ch] text-xl leading-relaxed md:text-2xl">
        {c.tryIt.body}
      </p>
      <div data-reveal className="relative mt-12 rounded-xl border border-line bg-card">
        <button
          type="button"
          onClick={onCopy}
          className="absolute top-3 right-3 cursor-pointer rounded-lg border border-line bg-night px-3 py-1.5 text-sm text-mist hover:border-signal hover:text-snow"
        >
          {copied ? c.tryIt.copied : c.tryIt.copy}
        </button>
        <pre className="overflow-x-auto px-5 py-6 pr-24 font-mono text-[0.85rem] leading-8 md:px-8 md:text-base">
          {INSTALL.map((line) => (
            <span key={line} className="block">
              <span className="text-signal select-none">$ </span>
              {line}
            </span>
          ))}
        </pre>
      </div>
      <ul className="mt-10 grid gap-6 md:grid-cols-2">
        {c.tryIt.after.map((a) => (
          <li key={a} data-reveal className="border-t border-snow/25 pt-4 leading-relaxed text-mist">
            {a}
          </li>
        ))}
      </ul>
    </Section>
  )
}

function Colophon({ c }: { c: Copy }) {
  return (
    <footer className="mt-10 mb-10 border-t-[3px] border-snow pt-8">
      <div className="grid gap-8 md:grid-cols-12">
        <div className="min-w-0 md:col-span-3">
          <LogoMark className="h-10 w-auto" />
        </div>
        <div className="md:col-span-6">
          <p className="smallcaps text-signal">{c.colophon.title}</p>
          <p className="mt-3 max-w-[48ch] leading-relaxed text-mist italic">{c.colophon.body}</p>
        </div>
        <ul className="smallcaps space-y-2 md:col-span-3 md:text-right">
          <li>
            <a className="underline decoration-snow/30 underline-offset-4 hover:text-signal" href={REPO}>
              {c.colophon.source} ↗
            </a>
          </li>
          <li>
            <a className="underline decoration-snow/30 underline-offset-4 hover:text-signal" href={`${REPO}/issues`}>
              {c.colophon.issues} ↗
            </a>
          </li>
          <li className="pt-3 text-mist">C2-POLY · 2026</li>
        </ul>
      </div>
    </footer>
  )
}

// The app's logo, in its own colours: a light wave and a blue caret.
function LogoMark({ className }: { className?: string }) {
  return (
    <svg viewBox="200 200 624 624" className={className} aria-hidden="true">
      <path
        d="M220 512 Q255 152 290 512 Q325 812 360 512 Q395 292 430 512 Q465 652 500 512 Q535 442 570 512 Q605 536 640 512 L690 512"
        fill="none"
        stroke="currentColor"
        strokeWidth="56"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <rect x="742" y="372" width="44" height="280" rx="22" className="fill-signal" />
    </svg>
  )
}

// A keycap as the app draws them: "⌥ droite", "⌘ droite".
function Key({ children }: { children: ReactNode }) {
  return (
    <kbd className="mx-0.5 inline-block rounded-md border border-[#3a4258] bg-[#151a27] px-2 py-px align-[0.08em] font-sans text-[0.85em] leading-snug font-normal text-snow not-italic shadow-[inset_0_-1px_0_rgba(255,255,255,0.06)]">
      {children}
    </kbd>
  )
}

// Copy strings mark keys as [⌥ droite]; this turns them into keycaps.
function rich(text: string) {
  return text.split(/\[([^\]]+)\]/).map((part, i) => (i % 2 ? <Key key={i}>{part}</Key> : part))
}

// A screenshot of the app, framed like its window and settling in as it scrolls.
function Shot({ src, width, height, caption }: { src: string; width: number; height: number; caption: string }) {
  return (
    <figure className="mt-16">
      <div data-plate className="overflow-hidden rounded-xl border border-line shadow-[0_50px_120px_-50px_rgba(0,0,0,0.95)]">
        <img
          src={`${import.meta.env.BASE_URL}${src}`}
          width={width}
          height={height}
          loading="lazy"
          alt={caption}
          className="block h-auto w-full"
          // Triggers below the picture were placed before it loaded.
          onLoad={() => ScrollTrigger.refresh()}
        />
      </div>
      <figcaption className="smallcaps relative mt-5 text-mist">{caption}</figcaption>
    </figure>
  )
}

function MenuBarIcon() {
  return (
    <svg viewBox="200 200 624 624" className="mt-1 h-4 w-auto shrink-0 text-snow" aria-hidden="true">
      <path
        d="M220 512 Q255 152 290 512 Q325 812 360 512 Q395 292 430 512 Q465 652 500 512 Q535 442 570 512 Q605 536 640 512 L690 512"
        fill="none"
        stroke="currentColor"
        strokeWidth="64"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <rect x="742" y="372" width="48" height="280" rx="24" fill="currentColor" />
    </svg>
  )
}

function SpeakerIcon() {
  return (
    <svg viewBox="0 0 20 20" className="h-3.5 w-3.5" aria-hidden="true">
      <path d="M3 8h3l4-3.5v11L6 12H3z" fill="currentColor" />
      <path d="M13 7.5a3.5 3.5 0 0 1 0 5M15.5 5a7 7 0 0 1 0 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  )
}
