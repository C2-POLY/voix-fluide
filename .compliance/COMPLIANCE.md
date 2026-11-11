# Compliance handoff: voix-fluide

For the owner of `C2-POLY/compliance-records`. Prepared by the compliance-init skill from the code of `C2-POLY/voix-fluide` at commit `3e7e03c8c879a1a1429eef54d3ad13cbbacb4865`. It prepares documentation; it does not certify compliance. The manifest of this repo is `.compliance/manifest.yaml`; once this pull request is merged, the workflow sends it to the compliance repo.

## 1. Register the repo

Add this entry to `repos` in `compliance.yaml`:

```yaml file=compliance.yaml
- name: voix-fluide
  url: https://github.com/C2-POLY/voix-fluide
  owner: developer and product owner of Voix Fluide
  expected_to_send: true
  components: [app, site]
```

## 2. Processing activities

The controller is the person using the app, so the organisation keeps no register of the processing done locally with their own accounts: the voice, the dictated text, the questions to Claude Code (sent with the person's own Claude Code account) and the signing certificate all stay on the person's Mac or go out under their own account.

Processing drafts below cover only what the organisation itself controls: none as YAML yet. The two candidates are under "To define", because their legal basis was not answered.

## 3. To define

- **Presentation site** (`site` component): GitHub Pages hosts `c2-poly.github.io/voix-fluide` for the organisation and sees each visitor's IP address and technical headers (manifest: `processors/github-pages`, `outbound_flows/site-hosting`). The site has no tracker and no external font. Needs a legal basis (likely to be discussed as legitimate interests), and a data processing agreement with GitHub: none is signed yet.
- **Update distribution** (`app` component): the organisation publishes the Releases that the app checks at launch and downloads from (manifest: `outbound_flows/update-check`). GitHub sees the IP address of each app that checks; no identifier is sent. Decide whether the organisation treats this as a processing of its own.

## 4. To complete in the compliance repo

- Controller contact, data protection officer, security measures of each processing (nothing in the code states them).
- The legal basis of the four `personal_data` entries, if the organisation wants to record one for local processing (question left open during compliance-init).
- The receiving side: the `receive` workflow and the App that reads this repo (see the compliance-action README), and CODEOWNERS for `services/**`. `C2-POLY/compliance-records` does not exist yet.
