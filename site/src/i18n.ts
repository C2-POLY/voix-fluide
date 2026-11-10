export type Lang = 'fr' | 'en'

export type Copy = {
  htmlTitle: string
  masthead: { issue: string; date: string; tag: string }
  nav: { how: string; local: string; pill: string; claude: string; try: string }
  hero: { lines: string[]; lede: string; keys: string[]; sentences: string[]; cue: string }
  how: {
    kicker: string
    title: string
    steps: { word: string; body: string }[]
    figure: string
    menubar: string
  }
  local: {
    kicker: string
    title: string
    body: string
    rows: [string, string][]
  }
  pill: {
    kicker: string
    title: string
    states: { label: string; caption: string }[]
    pasted: string
    figure: string
  }
  claude: {
    kicker: string
    title: string
    body: string
    qLabel: string
    question: string
    aLabel: string
    answer: string
    spoken: string
    points: string[]
    note: string
    figure: string
  }
  tryIt: {
    kicker: string
    title: string
    body: string
    after: string[]
    copy: string
    copied: string
  }
  colophon: { title: string; body: string; source: string; issues: string }
}

export const copy: Record<Lang, Copy> = {
  fr: {
    htmlTitle: 'Voix Fluide · dictée vocale pour macOS',
    masthead: { issue: 'N° 0.3', date: 'Novembre 2026', tag: 'Dictée vocale pour macOS' },
    nav: { how: 'Le geste', local: 'Le lieu', pill: 'La pastille', claude: 'Claude Code', try: 'L’essayer' },
    hero: {
      lines: ['Parlez.', 'Le texte', 's’écrit.'],
      lede: 'Une touche, votre voix, et la phrase arrive là où se trouve votre curseur. Dans Notes, dans un mail, dans le terminal. Sans compte, sans serveur : tout reste sur le Mac.',
      sentences: [
        'Rendez-vous jeudi à dix heures, au café.',
        'Pensez à relire le contrat avant midi.',
        'Le seuil passe à 0,4 seconde.',
      ],
      cue: 'Faites défiler',
      keys: ['Appuyez sur [⌥ droite] pour dicter.', 'Maintenez [⌘ droite] pour demander à Claude Code.'],
    },
    how: {
      kicker: 'I. Le geste',
      title: 'Trois temps, pas un de plus.',
      steps: [
        {
          word: 'Appuyez',
          body: 'Sur [⌥ droite]. En Bascule, un appui lance l’écoute et un second l’arrête ; en Maintien, elle dure tant que la touche est tenue ; en Automatique, au-delà de 0,4 s, c’est un appui long.',
        },
        {
          word: 'Dites',
          body: 'Parlez comme d’habitude. Une pastille noire s’affiche en bas de l’écran, sans jamais prendre le focus de l’app où vous écrivez.',
        },
        {
          word: 'C’est collé',
          body: 'Le texte arrive à l’endroit du curseur, par un [⌘ V]. Votre presse-papiers est remis tel qu’il était.',
        },
      ],
      figure: 'Fig. 1 · L’accueil de l’app : le bouton, et les deux touches.',
      menubar: 'Et dans la barre des menus, l’icône de Voix Fluide : l’état de l’app, la dernière dictée à copier, le choix du micro.',
    },
    local: {
      kicker: 'II. Le lieu',
      title: 'Tout se passe sur le Mac.',
      body: 'Le son est transcrit par un modèle qui tourne sur votre machine. Il est téléchargé une fois, puis plus rien ne sort : ni le son, ni le texte.',
      rows: [
        ['Whisper', 'Tiny, Base ou Small · 99 langues · sur la puce graphique, par Metal'],
        ['Parakeet TDT v3', '25 langues européennes, reconnues seules · environ 670 Mo · sur le processeur'],
        ['Vitesse', 'Une phrase de 5,6 s transcrite en 0,9 s environ, avec Parakeet'],
        ['Silence', 'Un enregistrement muet, ou de moins d’une seconde, n’est pas transcrit'],
        ['Mémoire', 'Les 50 dernières dictées restent dans la fenêtre, à recopier'],
      ],
    },
    pill: {
      kicker: 'III. La pastille',
      title: 'Elle écoute, puis elle s’efface.',
      states: [
        { label: 'Écoute', caption: 'Un point rouge, le niveau de votre voix et le temps écoulé.' },
        { label: 'Transcription…', caption: 'Le modèle travaille. Quelques fractions de seconde, en général.' },
        { label: 'Collé', caption: 'La phrase s’affiche un instant, puis la pastille disparaît.' },
      ],
      pasted: 'Bonjour, ceci est un essai de dictée vocale. Le texte apparaît dans la fenêtre.',
      figure: 'Fig. 2 · La pastille, à 50 points du bas de l’écran.',
    },
    claude: {
      kicker: 'IV. La seconde touche',
      title: 'Posez une question à votre code.',
      body: '[⌘ droite] enregistre une question au lieu d’une dictée. Nommez le dépôt dans la phrase : Voix Fluide lance Claude Code dans ce dossier, puis lit la réponse à voix haute.',
      qLabel: 'Vous, à voix haute',
      question: '« Dans voix fluide, où est réglé le seuil entre appui court et appui long ? »',
      aLabel: 'Claude Code, quelques secondes après',
      answer: 'Le seuil de 0,4 seconde est la constante TAP_HOLD_THRESHOLD dans engine.rs.',
      spoken: 'Lu à voix haute',
      points: [
        'Lecture seule : Claude Code peut lire et chercher, jamais modifier.',
        'Le dépôt est celui que vous nommez ; sinon, le dernier utilisé.',
        '« Reprendre au terminal » rouvre la session dans Claude Code, avec tout son contexte.',
      ],
      note: 'La dictée reste sur le Mac. La question transcrite, elle, part chez Claude Code.',
      figure: 'Fig. 3 · La page Claude Code de l’app, avec deux mini-sessions.',
    },
    tryIt: {
      kicker: 'V. L’essayer',
      title: 'Trois commandes.',
      body: 'Voix Fluide n’est pas encore distribuée : on la compile depuis les sources. Il faut macOS et rustup ; la bonne version de Rust s’installe d’elle-même.',
      after: [
        'Au premier lancement, macOS demande le micro, puis l’accessibilité pour pouvoir coller.',
        'Le modèle choisi se télécharge une seule fois.',
        'La première compilation prend quelques minutes, les suivantes quelques secondes.',
        'Pour la seconde touche, installez aussi Claude Code.',
      ],
      copy: 'Copier',
      copied: 'Copié',
    },
    colophon: {
      title: 'Colophon',
      body: 'Composé en Fraunces et IBM Plex Mono. L’app est écrite en Rust, avec egui ; ce site, avec React, GSAP et Lenis.',
      source: 'Code source',
      issues: 'Suivi des tickets',
    },
  },
  en: {
    htmlTitle: 'Voix Fluide · voice dictation for macOS',
    masthead: { issue: 'No. 0.3', date: 'November 2026', tag: 'Voice dictation for macOS' },
    nav: { how: 'The gesture', local: 'The place', pill: 'The pill', claude: 'Claude Code', try: 'Try it' },
    hero: {
      lines: ['Speak.', 'The text', 'writes itself.'],
      lede: 'One key, your voice, and the sentence lands where your cursor is. In Notes, in an email, in the terminal. No account, no server: everything stays on the Mac.',
      sentences: [
        'Meet me on Thursday at ten, at the café.',
        'Remember to read the contract before noon.',
        'The threshold is now 0.4 seconds.',
      ],
      cue: 'Scroll',
      keys: ['Press [⌥ Right] to dictate.', 'Hold [⌘ Right] to ask Claude Code.'],
    },
    how: {
      kicker: 'I. The gesture',
      title: 'Three beats, no more.',
      steps: [
        {
          word: 'Press',
          body: 'On [⌥ Right]. In Toggle, one press starts listening and a second one stops it; in Hold, it lasts as long as the key is down; in Automatic, anything over 0.4 s counts as a hold.',
        },
        {
          word: 'Say it',
          body: 'Talk as you always do. A black pill shows at the bottom of the screen and never takes focus from the app you are writing in.',
        },
        {
          word: 'Pasted',
          body: 'The text lands at the cursor, through a [⌘ V]. Your clipboard is put back the way it was.',
        },
      ],
      figure: 'Fig. 1 · The app’s home: the button, and the two keys (French interface).',
      menubar: 'And in the menu bar, the Voix Fluide icon: the app’s state, the last dictation to copy, the microphone to use.',
    },
    local: {
      kicker: 'II. The place',
      title: 'It all happens on the Mac.',
      body: 'Your voice is transcribed by a model running on your machine. It is downloaded once, then nothing leaves: neither the sound nor the text.',
      rows: [
        ['Whisper', 'Tiny, Base or Small · 99 languages · on the graphics chip, through Metal'],
        ['Parakeet TDT v3', '25 European languages, detected on its own · about 670 MB · on the processor'],
        ['Speed', 'A 5.6 s sentence transcribed in about 0.9 s, with Parakeet'],
        ['Silence', 'A silent recording, or one under a second, is not transcribed'],
        ['Memory', 'The last 50 dictations stay in the window, ready to copy'],
      ],
    },
    pill: {
      kicker: 'III. The pill',
      title: 'It listens, then it steps aside.',
      states: [
        { label: 'Listening', caption: 'A red dot, the level of your voice and the time so far.' },
        { label: 'Transcribing…', caption: 'The model at work. Usually a fraction of a second.' },
        { label: 'Pasted', caption: 'The sentence shows for a moment, then the pill is gone.' },
      ],
      pasted: 'Hello, this is a voice dictation test. The text appears in the window.',
      figure: 'Fig. 2 · The pill, 50 points above the bottom of the screen.',
    },
    claude: {
      kicker: 'IV. The second key',
      title: 'Ask your code a question.',
      body: '[⌘ Right] records a question instead of a dictation. Name the repository in the sentence: Voix Fluide runs Claude Code in that folder, then reads the answer aloud.',
      qLabel: 'You, out loud',
      question: '“In voix fluide, where is the threshold between a tap and a hold set?”',
      aLabel: 'Claude Code, a few seconds later',
      answer: 'The 0.4 second threshold is the TAP_HOLD_THRESHOLD constant in engine.rs.',
      spoken: 'Read aloud',
      points: [
        'Read only: Claude Code can read and search, never change anything.',
        'The repository is the one you name; otherwise, the last one used.',
        '“Reprendre au terminal” reopens the session in Claude Code, with all its context.',
      ],
      note: 'Dictation stays on the Mac. The transcribed question, however, goes to Claude Code.',
      figure: 'Fig. 3 · The app’s Claude Code page, with two mini-sessions (French interface).',
    },
    tryIt: {
      kicker: 'V. Try it',
      title: 'Three commands.',
      body: 'Voix Fluide is not distributed yet: you build it from source. You need macOS and rustup; the right Rust version installs itself.',
      after: [
        'On first launch, macOS asks for the microphone, then for Accessibility so it can paste.',
        'The model you pick downloads once.',
        'The first build takes a few minutes, the next ones a few seconds.',
        'For the second key, install Claude Code as well.',
      ],
      copy: 'Copy',
      copied: 'Copied',
    },
    colophon: {
      title: 'Colophon',
      body: 'Set in Fraunces and IBM Plex Mono. The app is written in Rust, with egui; this site, with React, GSAP and Lenis.',
      source: 'Source code',
      issues: 'Issue tracker',
    },
  },
}
