import { ReactLenis } from 'lenis/react'
import type { LenisRef } from 'lenis/react'
import { useEffect, useRef, type ReactNode } from 'react'
import { gsap, ScrollTrigger } from '../lib/gsap'
import 'lenis/dist/lenis.css'

export const reducedMotion = () =>
  typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches

export function SmoothScroll({ children }: { children: ReactNode }) {
  const lenisRef = useRef<LenisRef>(null)

  useEffect(() => {
    const lenis = lenisRef.current?.lenis
    lenis?.on('scroll', ScrollTrigger.update)
    const update = (time: number) => lenisRef.current?.lenis?.raf(time * 1000)
    gsap.ticker.add(update)
    gsap.ticker.lagSmoothing(0)
    return () => {
      lenis?.off('scroll', ScrollTrigger.update)
      gsap.ticker.remove(update)
    }
  }, [])

  if (reducedMotion()) return <>{children}</>

  return (
    <ReactLenis root options={{ autoRaf: false, lerp: 0.1, anchors: true }} ref={lenisRef}>
      {children}
    </ReactLenis>
  )
}
