# Minime

Minime est une application de bureau écrite en Rust avec [GPUI](https://gpui.rs/). Elle peut alléger une image ou la convertir vers un autre format sans modifier ses pixels.

Tout se passe localement : aucune image, statistique ou métadonnée n’est envoyée sur un serveur.

## Ce que fait Minime

- glisser-déposer et sélection native de plusieurs images ;
- compression et conversion par lot hors du thread d’interface ;
- introduction au premier lancement qui explique le flux et la garantie sans perte ;
- interface complète en anglais par défaut et en français, modifiable à tout moment ;
- thèmes système, clair et sombre avec choix persistant ;
- aperçu de l’image sélectionnée avec bascule entre l’original et le résultat Minime ;
- lecture directe du format, des dimensions, du poids et du gain obtenu ;
- mode `Auto` pour chercher le fichier exact le plus léger ;
- conversion vers `WebP lossless`, `PNG`, `QOI`, `TIFF`, `BMP` ou `Farbfeld` lorsqu’un format est choisi ;
- effort PNG/Auto réglable sur `Rapide`, `Équilibré` ou `Maximum` sans modifier les pixels ;
- destination à côté des originaux ou dans un dossier choisi ;
- protection optionnelle contre les sorties plus lourdes, activée en mode Auto et désactivée lorsqu’on choisit un format de conversion ;
- noms de sortie non destructifs : `photo.minime.webp`, puis `photo.minime-2.webp`, etc. ;
- comparaison pixel par pixel après réencodage et avant écriture ;
- conservation du profil ICC lorsque le format de sortie le permet ;
- détection des GIF, WebP et APNG animés pour ne jamais les aplatir silencieusement ;
- fenêtre compacte et adaptative pour macOS, Windows et Linux ;
- fond ivoire entièrement opaque et surfaces blanches sans transparence de fenêtre ;
- paramètres locaux persistants : langue, thème, format, destination, effort, aperçu et révélation du résultat ;
- lien de soutien [Buy me a coffee](https://buymeacoffee.com/scorpion7slayer) ;
- boutons animés au clic, indicateurs coulissants, apparitions séquencées et icônes d’état, sans animation décorative pendant la compression.

### Formats

Entrées : `PNG` / `APNG` statique, `JPEG` / `JFIF`, `WebP` statique, `GIF` statique, `BMP`, `TIFF`, `TGA`, `DDS`, `QOI`, `ICO`, `Farbfeld`, `PNM`, `PPM`, `PGM`, `PAM` et `PBM`.

| Sortie | Profondeur conservée | Profil ICC | Usage |
| --- | --- | --- | --- |
| Auto | 8 ou 16 bits | Oui | choisit le plus petit entre PNG et WebP lossless |
| PNG | 8 ou 16 bits | Oui | sortie universelle |
| WebP lossless | 8 bits | Oui | sortie compacte moderne |
| QOI | 8 bits | Non | encodage et décodage rapides |
| TIFF | 8 ou 16 bits | Oui | archivage et production |
| BMP | 8 bits | Non | compatibilité avec les outils anciens |
| Farbfeld (`.ff`) | RGBA 16 bits | Non | format ouvert et très simple |

Les images animées sont refusées dans cette première version. Une sortie qui ne peut pas préserver la profondeur ou le profil colorimétrique est refusée avec une erreur explicite. AVIF et JPEG ne sont pas proposés en sortie, car leur réencodage courant est avec perte.

## “Sans perte” dans Minime

Minime applique une définition stricte : les dimensions et toutes les valeurs RGBA 16 bits obtenues après décodage doivent être identiques avant et après compression. Si cette vérification échoue, le fichier n’est pas écrit.

Lors d’une conversion, l’orientation EXIF est appliquée aux pixels pour conserver l’apparence. Le profil colorimétrique ICC est copié vers PNG, WebP et TIFF. Les autres métadonnées non visuelles ne sont pas garanties lors d’une conversion de format.

Le réglage d’effort agit sur la recherche de compression PNG, pas sur la qualité visuelle. `Maximum` peut prendre plus de temps mais conserve la même vérification exacte que les autres modes.

## Paramètres locaux

Minime n’utilise aucun compte ni service distant. Les préférences sont enregistrées dans le dossier de configuration standard du système (`Application Support` sur macOS, `AppData` sur Windows et `XDG_CONFIG_HOME` ou `~/.config` sur Linux).

## Raccourcis

| Action | macOS | Windows / Linux |
| --- | --- | --- |
| Ajouter des images | `⌘O` | `Ctrl+O` |
| Alléger ou convertir | `⌘Entrée` | `Ctrl+Entrée` |
| Vider la file | `⌘⇧K` | `Ctrl+⇧K` |

## Lancer en développement

Prérequis : Rust 1.88 ou plus récent et les dépendances système demandées par GPUI.

```bash
cargo run
```

Sur Ubuntu/Debian, les bibliothèques de développement usuelles sont :

```bash
sudo apt-get install libasound2-dev libfontconfig1-dev libwayland-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev
```

GPUI utilise ici ses shaders Metal compilés au lancement sur macOS (`runtime_shaders`). Cela évite d’exiger le composant Metal Toolchain de Xcode pour compiler Minime localement.

## Construire

```bash
cargo build --release
```

Le binaire se trouve dans `target/release/minime` (`minime.exe` sous Windows).

Pour créer une application macOS :

```bash
./scripts/package-macos.sh
open dist/Minime.app
```

Les workflows GitHub Actions vérifient macOS, Windows et Linux. Un tag `v*` construit aussi les archives de chaque plateforme et les joint à la GitHub Release correspondante.

## Architecture

- `src/compression.rs` : détection, décodage orienté, encodeurs, optimisation PNG, vérification et écriture atomique ;
- `src/main.rs` : interface GPUI, introduction, aperçu, paramètres, glisser-déposer et exécution asynchrone ;
- `src/localization.rs` et `src/preferences.rs` : anglais/français et préférences multiplateformes ;
- `packaging/` et `scripts/` : empaquetage par plateforme ;
- `.github/workflows/` : validation multiplateforme et releases.

## Vérifier

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
```

Les tests couvrent notamment l’égalité des pixels des sorties QOI, TIFF, BMP et Farbfeld, la réduction effective d’un PNG non optimisé, la génération de noms sans écrasement, les deux formats de taille et la sérialisation des préférences.

## Licence

[MIT](LICENSE)
