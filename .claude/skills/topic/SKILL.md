---
name: topic
description: Workflow obligatoire pour tout nouveau sujet sur Voix Fluide (fonctionnalité, amélioration ou bug) : analyse, issue GitHub dans le Project Voix Fluide (avec un design et un schéma Mermaid validés quand le sujet a un flux ou est complexe), worktree, développement, tests, test visuel, test du dev en dictant, puis commit, push, PR, statut du Project et rangement. À utiliser dès que le dev demande d'ajouter, de changer ou de corriger quelque chose, ou tape /topic. Pas pour une simple question, ni pour continuer un sujet déjà en cours.
---

# Topic : un nouveau sujet

Chaque sujet suit cet ordre, sans en sauter une étape. **Un sujet = une issue = un worktree = une PR.** Si le dev glisse un deuxième sujet en route, on le note et on le traite après, séparément.

Le suivi se fait dans GitHub Issues et le Project **Voix Fluide**. Les règles du dépôt (commandes, architecture, issues, version) sont dans `CLAUDE.md`. Le dépôt est le dossier `app/` de l'espace de travail : les chemins ci-dessous partent de sa racine (`src/`, `Cargo.toml`), sans `app/`.

Montre l'avancement avec les chapitres de la section suivante, et rien d'autre : ni barre de progression, ni liste du parcours dans tes messages. Si une étape est incomplète, on ne passe pas à la suivante.

## Avancement

Les huit étapes portent toujours ces noms courts : Analyse, GitHub, Worktree, Développement, Tests, Visuel, Essai du dev, Clôture.

À chaque début d'étape, marque un chapitre avec `mcp__ccd_session__mark_chapter`, au titre exact `N/8 Étape` (par exemple `5/8 Tests`). Le titre ne porte rien d'autre : ni numéro d'issue, ni parenthèse, et pas de `summary`.

- **Si une étape bloque**, ne change pas de chapitre : dis en une phrase ce qui bloque, avec la sortie.
- **Si on revient en arrière** (l'essai du dev échoue à l'étape 7, on corrige à l'étape 4), marque de nouveau le chapitre de l'étape où l'on revient.
- **Sans cet outil** (hors de l'app Claude), écris seulement `N/8 Étape` en première ligne du message, au début de chaque étape.

## Niveau du sujet : modèle et effort

Chaque sujet a un niveau, Simple, Standard ou Complexe, qui choisit le modèle et l'effort des sessions à part de ses étapes. Le niveau se règle à l'étape 1 et s'annonce en une ligne (« Niveau : Standard, Sonnet à effort medium pour le développement »). Le dev peut le changer à tout moment, et un sujet qui révèle en route un risque (perte de dictée, permission perdue, texte collé au mauvais endroit) monte de niveau, annoncé de la même façon.

**Critères**, le premier qui convient l'emporte :

- **Complexe** : touche les fils et le partage d'état (`controller`, `worker`, `Cmd`, `Job`, `Shared` et son `Mutex` dans `engine.rs`), le micro (`open_mic`, `cpal`), le décodeur Whisper ou le chargement d'un modèle (`whisper.rs`, `Engine`, Metal), les permissions et la relance (`run_with_own_permissions`, `stable_copy`, la signature, `build.rs` et son Info.plist), ce qui est collé dans l'app active ou mis dans le presse-papiers (`paste_text`, `post_paste`), ce qui est écrit chez l'utilisateur (`settings.json`, `models/`, `programme/`), un bug dont on ne trouve pas la cause, ou plus de quatre fichiers de code.
- **Standard** : une fonctionnalité ou un bug localisé dans la fenêtre, la pastille ou les réglages, de deux à quatre fichiers de code, qui ne touche ni aux fils ni à macOS.
- **Simple** : docs, skill, outillage, textes, une constante ou une couleur, un ou deux fichiers.

**Le modèle et l'effort par étape** (`model` et `effort` de la session à part, `mcp__ccd_session__start_session`) :

| Étape | Simple | Standard | Complexe |
|---|---|---|---|
| 1 Analyse (lecture du code) | Haiku, low | Haiku, low | Sonnet, medium |
| 2 GitHub | session principale | session principale | session principale |
| 3 Worktree | session principale | session principale | session principale |
| 4 Développement | Haiku, low | Sonnet, medium | Opus, high |
| 5 Tests (tri des échecs) | session principale | Haiku, low | Sonnet, medium |
| 6 Visuel | session principale | session principale | session principale |
| 7 Essai du dev | session principale (le dev essaie) | session principale | session principale |
| 8 Commit, PR, Project | session principale | session principale | session principale |

Règles de délégation :

- **Déléguer, c'est ouvrir une session à part, jamais lancer un agent en arrière-plan** : une étape déléguée part dans un sous-chat que le dev voit dans la barre latérale, peut ouvrir, suivre et arrêter (`mcp__ccd_session__start_session`, `initiation` « user_asked », `context` « fresh », `use_worktree` false avec un brief qui dit de travailler dans le worktree du sujet). Un agent en arrière-plan (l'outil Agent, `run_in_background`) ne se lance que si le dev le demande explicitement.
- **Une session à part ne voit pas la conversation** : son brief dit le but, le worktree et les fichiers, ce qui est déjà fait, ce qui est exclu, ce qu'il doit rendre et ce qu'il ne fait pas (ni commit, ni push, ni tests « live »).
- **L'issue, le choix des tests, l'essai du dev, le commit et la PR restent dans la session principale**, qui reprend la main quand la session à part a fini.
- **Le rapport d'une session à part est une indication, pas une preuve** : relis le diff, les fichiers et lignes cités, les sorties de tests avant de t'en servir.
- **Le coût suit le niveau** : Opus ne se prend que pour le niveau Complexe. Pour un doute sur le niveau, prends le plus bas et monte si la tâche le demande.

## 1. Analyser

Lis le code et les docs concernés (`CLAUDE.md`, `MECHANICS.md` si le sujet touche une valeur mesurée sur l'app d'origine, les fichiers de `src/` touchés). Reformule le sujet en deux ou trois phrases. Aucun code. Classe le sujet selon « Niveau du sujet » et annonce le niveau, puis dis en une ligne s'il demande un design (voir « Design » à l'étape 2) : « Design : oui, sequenceDiagram du raccourci au collage ».

## 2. Chercher dans GitHub

Cherche d'abord une issue existante, ouverte ou fermée, dans le dépôt et dans le Project :

```bash
gh issue list --repo C2-POLY/voix-fluide --state all --search "<mots>" --limit 30
gh project item-list 5 --owner C2-POLY --format json --limit 300 --jq '.items[] | "\(.status)\t#\(.content.number // "-")\t\(.title)"'
```

Puis :

- **Elle existe** : lis-la (`gh issue view <n> --repo C2-POLY/voix-fluide --comments`), résume-la au dev, vérifie que ses critères d'acceptation sont encore justes. Une issue fermée et livrée ne se rouvre pas : si le sujet la reprend, on définit une nouvelle issue qui la cite (« Reprend #12 »).
- **Elle n'existe pas** : on la définit ensemble, **avant tout code**. Pose au dev les questions ouvertes : comportement attendu, cas limites (dictée très courte, silence, micro Bluetooth, modèle pas encore téléchargé, permission refusée), ce qui est exclu, ce qui peut être perdu ou collé au mauvais endroit. Ne devine pas. Rédige l'issue selon « Work items » de `CLAUDE.md` (français, titre `<Zone> : …`, critères en cases à cocher, type, label `zone:`), fais-la valider, puis crée-la et mets-la dans le Project :

```bash
# Le type (Feature, Task, Bug) se donne à la création par l'API ; --body-file évite les problèmes de guillemets.
url=$(gh api repos/C2-POLY/voix-fluide/issues -f title="<titre>" -F body=@<fichier du corps> -f type=Task -f 'labels[]=zone:<zone>' -f "assignees[]=$(gh api user --jq .login)" --jq .html_url)
# Garde l'id de l'item : `item-list` ne montre pas tout de suite un item qui vient d'être ajouté.
item=$(gh project item-add 5 --owner C2-POLY --url "$url" --format json --jq .id)
```

Un label `zone:` qui manque se crée (`gh label create zone:<zone> --repo C2-POLY/voix-fluide`) plutôt que d'être omis. Un sujet découpé est une Feature avec des sous-issues ; une dépendance est une relation `blocked by`, pas une phrase.

**Changer un statut** (les identifiants sont dans `CLAUDE.md`, « Work items ») :

```bash
# Sans id gardé à l'étape 2 (issue ajoutée plus tôt), cherche-le dans le Project.
[ -n "$item" ] || item=$(gh project item-list 5 --owner C2-POLY --format json --limit 300 --jq ".items[] | select(.content.number == <n>) | .id")
gh project item-edit --project-id PVT_kwDODd-WqM4Bq3jr --id "$item" --field-id PVTSSF_lADODd-WqM4Bq3jrzhpMuYM --single-select-option-id <option>
```

### Design, quand le sujet a un flux ou est complexe

**Quand** : toujours pour un sujet Complexe ; pour un sujet Standard dès qu'il y a un flux (plusieurs étapes qui s'enchaînent, un message entre les fils, un téléchargement, une phase de la dictée qui change) ; et chaque fois que le dev le demande. Jamais pour un sujet Simple.

**Quoi** : une section `## Design` dans le corps de l'issue, sous les critères, rédigée **avant tout code** :

- **Fichiers** : ceux qu'on touche, et pourquoi, en une ligne chacun.
- **Approche** : trois à cinq lignes.
- **Schéma** : un bloc ` ```mermaid `, un seul sauf si le sujet a vraiment deux flux distincts, choisi selon ce qui risque de casser :
  - `sequenceDiagram` pour les fils et canaux (`controller`, `worker`, `Cmd`, `Job`, le rappel `cpal`) et pour le chemin d'une dictée, du raccourci au collage : qui envoie quoi, dans quel ordre, ce qui se passe si le modèle n'est pas prêt ou si le micro ne donne que du silence ;
  - `stateDiagram-v2` pour les phases de la dictée (`Phase`), l'état d'un modèle (`ModelState`), les permissions, ou la pastille qui a plusieurs états ;
  - `flowchart` pour un flux de données : le son lu, rééchantillonné, transcrit, nettoyé, collé, ou ce qui est écrit (`settings.json`, `models/`) ou téléchargé.
  Le schéma décrit le code tel qu'il sera, avec les vrais noms de fonctions, de types et de messages, pas une intention.
- **Risques** : ce qui peut être perdu (une dictée, le presse-papiers du dev), collé au mauvais endroit, ou écrit chez l'utilisateur, et comment on le reprend (l'historique, le presse-papiers remis).
- **Tests** : ce que l'étape 5 vérifiera, et ce que le dev essaiera à l'étape 7.

**Validation** : montre la section dans ton message (le bloc `mermaid` s'affiche en schéma dans l'app) et attends l'accord du dev avant de créer l'issue. Si l'issue existe déjà, ajoute la section à son corps (`gh issue edit <n> --repo C2-POLY/voix-fluide --body-file <fichier>`) puis donne-lui le lien pour qu'il la relise affichée sur GitHub. Pas de worktree tant que le design n'est pas validé.

**Tenir le design juste** : si le développement s'en écarte (une autre approche, un message de plus, un fichier imprévu), mets la section à jour dans l'issue et dis au dev en une ligne ce qui a changé. Un design faux est pire que pas de design.

### Conformité, une fois l'issue validée

Le dépôt a un manifest de conformité, `.compliance/manifest.yaml` : ce que l'app fait des données personnelles (voix, texte dicté, questions à Claude Code…), où elles sont gardées, à qui elles partent, ses fonctions d'IA. Une fois l'issue créée ou relue (et son design validé, s'il y en a un), lance le skill `compliance-step` sur elle. Il lit l'issue et le manifest, regarde six déclencheurs (une donnée personnelle nouvelle, un service ou destinataire externe, un stockage ou une durée de conservation, une décision automatisée ou une fonction d'IA, ce qu'on dit aux personnes — page de confidentialité, écran, message —, le rôle ou le niveau de risque AI Act d'une fonction), et ne pose que les questions de ceux qui se déclenchent, en un seul message numéroté.

- **Sa section va à la fin du corps de l'issue** (`gh issue edit <n> --repo C2-POLY/voix-fluide --body-file <fichier>`), en français : `**Conformité**` avec une ligne par déclencheur, ou les deux lignes `Impact RGPD : aucun` et la raison. Le manifest est au format v0.2 et ses textes libres (descriptions, finalités, champs, durées) s'écrivent en français, la langue du dossier (`organisation.language: fr` dans `C2-POLY/compliance-records`) ; les ids et les valeurs d'énumération restent en anglais. Le responsable de traitement dépend du composant (`service.components`) : pour une donnée nouvelle de `app`, c'est l'utilisateur, et `compliance-step` ne demande pas de base légale (`legal_basis: not-applicable`) ; pour `updates` et `site`, c'est C2-POLY, et il la demande avec son détail, qui va dans la section Conformité pour la fiche de traitement.
- **Pas de worktree tant que ses questions n'ont pas de réponse.** Une question sans réponse reste une ligne `TODO:` dans la section, jamais une valeur devinée.
- **Le manifest se change dans le worktree, à l'étape 4**, pas maintenant : garde les réponses jusque-là.

Ce travail prépare la documentation, il ne certifie rien. Après le merge, le workflow `compliance.yml` envoie le manifest à `C2-POLY/compliance-records`, où une PR attend sa relecture.

## 3. Worktree

Toujours un worktree, et tu ne demandes rien au dev, ni le choix ni le nom : plusieurs sessions Claude peuvent partager le dossier principal et changer la branche sous tes pieds. Choisis le nom toi-même, `feat/<n>-…` pour une Feature, `fix/<n>-…` pour un Bug, `chore/<n>-…` pour l'outillage et la doc (`<n>` le numéro de l'issue), et crée-le à côté de l'espace de travail, depuis `origin/main` et sans qu'il suive `main`, pour qu'un `git push` nu ne parte jamais vers `main`. Depuis le dossier principal (`…/voix-fluide/app`) :

```bash
git fetch --prune && git worktree add -b <type>/<n>-<nom> ../../voix-fluide-<nom> origin/main && git -C ../../voix-fluide-<nom> branch --unset-upstream
```

Un worktree neuf n'a pas de `target/` : copie celui du dossier principal pour éviter une compilation complète (candle et ort sont longs), `cp -Rc <dossier principal>/target ../../voix-fluide-<nom>/` (instantané sur APFS). Passe l'issue à **In progress**, dis en une ligne le chemin et la branche, puis passe à l'étape 4. Avant chaque commit, revérifie la branche avec `git branch --show-current`.

## 4. Travailler

Respecte `CLAUDE.md` (architecture, fils et canaux, version) et, s'il y en a un, le design validé de l'issue : le brief d'une session à part le cite en entier, schéma compris. La fenêtre ne bloque jamais sur le micro ou le modèle : ce travail passe par `Cmd` et `Job`, et le verrou de `Shared` se tient le moins longtemps possible. Ne touche pas au presse-papiers du dev sans le remettre quand le réglage le demande. Ajoute ou mets à jour les tests unitaires (`#[cfg(test)]` dans le fichier touché, pour la logique pure : `resample`, `loudness`, `clean`, `pill_layout`, les réglages ; un test qui télécharge un modèle ou transcrit pour de vrai est `#[ignore]`). Quand le sujet change quelque chose de visible dans l'app, applique la règle de version du jour (« Versioning »). Si une valeur de `mechanics.json` change, mets `MECHANICS.md` à jour. Si la conformité de l'étape 2 a déclenché quelque chose, mets `.compliance/manifest.yaml` à jour avec les réponses, comme le dit `compliance-step` : chaque élément ajouté ou changé prend `source: {kind: spec, ref: <url de l'issue>}`, et les fichiers touchés vont dans `linked_files`. Le développement se délègue au modèle et à l'effort du tableau de « Niveau du sujet ».

## 5. Tester

Depuis la racine du worktree :

```bash
# Compiler : aucun warning
cargo build 2>&1 | grep -E '^(warning|error)'

# Tests unitaires : tous doivent passer
cargo test

# Lints : aucun warning
cargo clippy 2>&1 | grep -E '^(warning|error)' | sort | uniq -c

# Seulement si le sujet touche la transcription, les modèles ou le son : les tests qui téléchargent et transcrivent pour de vrai
cargo test -- --ignored --nocapture

# Et une transcription de bout en bout sur un WAV 16 bits (le texte attendu doit sortir)
cargo run -- --transcribe <fichier.wav> fr

# Seulement si le manifest a changé : il doit passer le validateur de compliance-spec, à jour
# (un schéma plus ancien que le manifest refuse ses champs v0.2 récents ; le script choisit le schéma selon spec_version)
spec="${COMPLIANCE_SPEC:-$HOME/Documents/c2-poly/repo/compliance/compliance-spec}"
[ -d "$spec/.git" ] || spec="${TMPDIR:-/tmp}/compliance-spec"
if [ -d "$spec/.git" ]; then git -C "$spec" pull -q --ff-only; else git clone -q --depth 1 https://github.com/C2-POLY/compliance-spec "$spec"; fi
(cd "$spec" && npm ci --silent)
node "$spec/scripts/validate.mjs" .compliance/manifest.yaml
```

Un test qui échoue s'explique par le sujet ou se corrige ; ne le désactive pas. Un warning nouveau se corrige. En cas de doute, lance la même commande dans le dossier principal, sur `main`, pour comparer.

## 6. Tester visuellement

Vérifie ce que le sujet change à l'écran : capture de la fenêtre et de la pastille, lancées depuis `/tmp`, puis lis l'image.

```bash
S=<scratchpad>
(cd /tmp && <worktree>/target/debug/voix-fluide --screenshot "$S/fenetre.png")
(cd /tmp && VF_PILL_SHOT="$S/pastille.png" <worktree>/target/debug/voix-fluide --screenshot /dev/null)
```

La capture de la pastille montre ses états les uns sous les autres (écoute, collé avec le texte, transcription, compacte) ; garde `--screenshot`, sans quoi l'app se relance pour ses permissions. Si le sujet touche une vue ou un état qu'aucune capture ne montre, ajoute un réglage du même genre (`VF_…`) dans `App::new`. Recadre avec `sips -c <h> <w> --cropOffset <y> <x>` pour regarder un détail. Contrôle les glyphes (un carré veut dire que la police ne l'a pas), les textes coupés et les coins arrondis transparents de la pastille. Dis ce que tu as vu et ce que tu n'as pas pu voir (la pastille par-dessus une autre app, une vraie dictée, une demande de permission).

## 7. Faire essayer le dev, avec lecture du terminal

Ne lance pas l'app toi-même pour lui : une dictée demande sa voix, son micro et ses permissions. Donne au dev la commande pour la lancer **depuis le worktree**, en un seul bloc `bash`, plus une ligne sur ce qu'il doit essayer (par exemple : dicter une phrase dans Notes avec la touche du raccourci, en appui puis en tapotant) :

```bash
cd <chemin absolu du worktree> && cargo run
```

Le chemin est celui du worktree, vérifié avec `git worktree list` avant l'envoi : ne reprends jamais celui d'une commande précédente. Garde le `cd` : rustup choisit la version de Rust selon le dossier courant, et un `cargo run --manifest-path` lancé d'ailleurs prend une autre version et recompile tout (voir « Commands » de `CLAUDE.md`). Si son terminal montre un autre chemin, dis-le tout de suite. Le programme copié et signé dans `VoixFluide/programme/` se refait quand le binaire change : si macOS redemande une permission, c'est attendu après un changement de `mac.rs` ou de `build.rs`, pas un bug.

Quand il a essayé et fermé l'app, lis son terminal (outil `read_terminal`, onglet principal ; sinon demande-lui de coller la sortie). Cherche les `panicked`, les `error`, les `warning` du build et tout message inattendu, puis demande-lui ce qu'il a vu et ce qui a été collé : un terminal propre ne dit pas ce qu'il a vu, donc attends son verdict avant de commiter. Corrige et reteste jusqu'à ce que tout soit propre.

## 8. Clôturer, seulement si aucune erreur

1. Avant le commit, relance `compliance-step` sur le diff (`git diff origin/main...HEAD`) : le code peut faire plus que ce que disait l'issue (un champ gardé, un appel réseau nouveau). Si un déclencheur part que l'étape 2 n'avait pas vu, pose ses questions, mets le manifest à jour, revalide-le (étape 5) et remplace la section Conformité de l'issue. `.compliance/manifest.yaml` fait alors partie du commit.
   Commit sur la branche du sujet (message en anglais, Conventional Commits : `feat(pill): …`, sans numéro de version). Jamais de relecture du diff par un agent avant le commit : ses tests et l'essai du dev sont le filet.
2. Push de la branche, puis PR vers `main` (titre en anglais, Conventional Commits ; corps avec `Closes #<n>`). L'issue passe à **In review**.
3. Issue : vérifie que la section Design, s'il y en a une, décrit bien le code livré (sinon mets-la à jour), coche les critères que le dev a vérifiés à l'étape 7, ajoute la ligne `**Livré** : \`vX.Y.Z\`` et le label de version. Une fois la PR mergée, passe l'issue à **Done**. Si un critère n'a pas pu être testé, l'issue va à **To test** avec sa case décochée, et tu dis lequel.
4. Une fois la PR mergée dans `main`, déplace le tag de version (voir « Versioning » de `CLAUDE.md`).
5. **Worktree du sujet, sans le redemander.** Dès que la PR est mergée, supprime le worktree du sujet et sa branche locale. Vérifie d'abord, depuis le dossier principal et non depuis le worktree à supprimer :
   - `git fetch --prune`, puis la branche est bien dans `origin/main` (`git branch -r --merged origin/main`, ou la PR « merged » si elle a été squashée) ;
   - `git -C <worktree> status --short` ne montre rien.

   Puis `git worktree remove <chemin du worktree>` et `git branch -D <branche>` (`-D` seulement si la PR a été squashée et que GitHub la dit mergée). Si le worktree a du travail non commité ou si la branche n'est pas mergée, ne supprime rien : dis ce qui bloque, avec la sortie, et demande.
6. **Rangement du reste, en un seul message.** Propose tout ce qui reste, jamais la moitié :
   - la branche distante du sujet, si elle existe encore (`git fetch --prune`) ;
   - `main` du dossier principal à jour (`git merge --ff-only origin/main`, seulement s'il est sur `main` et propre) ;
   - tes fichiers temporaires (captures, WAV, journaux du scratchpad). Écris `"${S:?}"/*.png` plutôt qu'une variable nue ;
   - les autres worktrees et branches locales déjà mergés dans `main` : regarde le `git status` de chacun, et ne touche jamais à un dossier avec du travail non commité ni à une branche distante qui n'est pas la tienne ;
   - une issue (Backlog) pour chaque défaut remarqué en route et laissé de côté ;
   - les issues liées encore ouvertes alors que leur travail est fini.

S'il reste une erreur, on ne commit pas et on ne ferme pas l'issue : tu dis ce qui échoue, avec la sortie.
