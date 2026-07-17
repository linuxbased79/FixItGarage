# Account setup guide — FixItGarage

You need a few free accounts so **Play**, **F-Droid**, **donations**, and the **website** all work.  
Do them in this order if you want the fastest path to “live.”

| Account | Why | Time | Cost |
|---------|-----|------|------|
| **GitHub Pages** | App website + donate page | ~5 min | Free |
| **Liberapay** (or Ko-fi / Sponsors) | Donate button destination | ~15 min | Free |
| **Google Play Console** | Publish on Play | ~30–60 min | **$25 one-time** |
| **GitLab.com** | F-Droid `fdroiddata` merge request | ~15 min | Free |

Website (after Pages is on):  
**https://linuxbased79.github.io/FixItGarage/**

---

## 1. Website (GitHub Pages) — free

Repo already contains `docs/` (homepage + donate page).

### Enable Pages (web UI)

1. Open https://github.com/linuxbased79/FixItGarage/settings/pages  
2. **Source:** Deploy from a branch  
3. **Branch:** `main`  
4. **Folder:** `/docs`  
5. Save  

After 1–2 minutes:

- Home: https://linuxbased79.github.io/FixItGarage/  
- Donate: https://linuxbased79.github.io/FixItGarage/donate.html  

### Enable Pages (CLI, if you prefer)

```bash
gh api -X POST repos/linuxbased79/FixItGarage/pages \
  -f build_type=legacy \
  -f source[branch]=main \
  -f source[path]=/docs
```

Set the repo homepage (optional):

```bash
gh repo edit linuxbased79/FixItGarage --homepage "https://linuxbased79.github.io/FixItGarage/"
```

---

## 2. Donations — make the Donate button useful

The app opens:

**https://linuxbased79.github.io/FixItGarage/donate.html**

That page lists Liberapay, GitHub Sponsors, Ko-fi, and PayPal.Me.  
**You must create at least one provider** or the buttons will 404 / land on signup.

### Option A — Liberapay (recommended for FOSS)

1. Go to https://liberapay.com/sign-up  
2. Sign up (email or GitHub).  
3. Create your profile; prefer username **`linuxbased79`** so the link matches,  
   or change the href in `docs/donate.html` to your real Liberapay URL.  
4. Connect a payout method (bank / Stripe / PayPal depending on region).  
5. Open https://liberapay.com/YOURNAME and confirm it loads.  
6. On a phone: open the donate page → **Donate on Liberapay** → complete a $1 test if you want.

### Option B — GitHub Sponsors

1. https://github.com/sponsors → **Join the waitlist / Enable Sponsors** (availability varies by country).  
2. Complete Stripe identity / payout.  
3. Your page will be: https://github.com/sponsors/linuxbased79  

### Option C — Ko-fi (easiest one-time tips)

1. https://ko-fi.com/ → Sign up.  
2. Set page name (e.g. `linuxbased79`).  
3. Update `docs/donate.html` button if the username differs.  

### Option D — PayPal.Me

1. https://www.paypal.com/paypalme/  
2. Create `paypal.me/YourName`  
3. Update the PayPal link in `docs/donate.html` if needed.

**Minimum to “work”:** Liberapay **or** Ko-fi **or** PayPal.Me live, then tap **Donate** in the app.

---

## 3. Google Play Console — ~$25 once

### Create the developer account

1. Use a Google account you control long-term.  
2. Open https://play.google.com/console/signup  
3. Accept agreements, pay the **one-time $25** registration fee.  
4. Complete identity verification (can take hours–days).  

### Create the app

1. **Create app** → name **FixItGarage**, language English, Free, declare declarations.  
2. Dashboard → complete:

| Section | What to use |
|---------|-------------|
| Privacy policy | `https://raw.githubusercontent.com/linuxbased79/FixItGarage/main/PRIVACY.md` |
| App category | Auto & Vehicles / Tools |
| Contact email | yours |
| Store listing | text in `metadata/en-US/` |
| Screenshots | `metadata/en-US/images/phoneScreenshots/` |
| Feature graphic | `metadata/en-US/images/featureGraphic/featureGraphic.png` |
| Icon | `metadata/en-US/images/icon/icon.png` |
| Data safety | No developer-side collection; local data; optional user share |
| Content rating | Questionnaire (utility) |

### Signing (do once, keep safe)

```bash
cd rust
./scripts/create-upload-keystore.sh ~/fixitgarage-upload.jks upload
export FIG_KEYSTORE=$HOME/fixitgarage-upload.jks
export FIG_KEYSTORE_PASS='your-password'
export FIG_KEY_ALIAS=upload
export FIG_KEY_PASS='your-password'
./scripts/release-apks.sh
```

Upload **arm64** APK to **Internal testing** first. Enable **Play App Signing**.

Full detail: [`PLAY.md`](PLAY.md) · overview: [`STORE.md`](STORE.md)

---

## 4. GitLab.com — for F-Droid

F-Droid metadata lives on **GitLab**, not GitHub.

1. Create account: https://gitlab.com/users/sign_up  
   (You can “Sign in with GitHub”.)  
2. Fork https://gitlab.com/fdroid/fdroiddata  
3. In your fork, add file:

   `metadata/org.fixitgarage.app.yml`  

   Copy from this repo: [`metadata/org.fixitgarage.app.yml`](metadata/org.fixitgarage.app.yml)

4. Ensure GitHub tag **`v0.2.20`** (or newer) exists and matches `commit:` in the yml.  
5. Open a **Merge Request** to `fdroid/fdroiddata` with a short description + link to source.  
6. Watch the MR for bot/build comments; fix recipe if asked.

Guide: [`F-DROID.md`](F-DROID.md)

---

## 5. Checklist

- [ ] GitHub Pages → `/docs` on `main`  
- [ ] Visit website + donate page on phone  
- [ ] Liberapay (or Ko-fi / Sponsors / PayPal) **live**  
- [ ] App **Donate** opens donate page and a payment option  
- [ ] Play Console registered + app created  
- [ ] Upload keystore created offline (not in git)  
- [ ] Internal testing track has a signed APK  
- [ ] GitLab account + fdroiddata fork + MR  

---

## Support links (after setup)

| What | URL |
|------|-----|
| Website | https://linuxbased79.github.io/FixItGarage/ |
| Donate | https://linuxbased79.github.io/FixItGarage/donate.html |
| Source | https://github.com/linuxbased79/FixItGarage |
| Issues | https://github.com/linuxbased79/FixItGarage/issues |
| Privacy | https://raw.githubusercontent.com/linuxbased79/FixItGarage/main/PRIVACY.md |
| Releases | https://github.com/linuxbased79/FixItGarage/releases |

When a donation username differs from `linuxbased79`, edit the four buttons in [`docs/donate.html`](docs/donate.html) and commit — the app still opens the same donate page.
