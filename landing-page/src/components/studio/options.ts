export const groups = [
  {
    id: "hero",
    label: "Haut de page",
    options: [
      { value: "centre", label: "Centré, iPhone dessous" },
      { value: "cote", label: "Texte à gauche, iPhone à droite" },
      { value: "geant", label: "Titre géant" },
    ],
  },
  {
    id: "promo",
    label: "Vidéo du haut",
    options: [
      { value: "noire", label: "Ouverture noire" },
      { value: "orange", label: "Ouverture orange" },
    ],
  },
  {
    id: "herobg",
    label: "Fond du haut",
    options: [
      { value: "blanc", label: "Blanc pur" },
      { value: "orange", label: "Bloc orange plein" },
    ],
  },
  {
    id: "orange",
    label: "Orange",
    options: [
      { value: "pur", label: "#FF5C00 pétant" },
      { value: "clair", label: "#FF7A00 lumineux" },
      { value: "logo", label: "#DD5F1C du logo" },
    ],
  },
  {
    id: "type",
    label: "Typo des titres",
    options: [
      { value: "switzer", label: "Switzer" },
      { value: "clash", label: "Clash Grotesk" },
      { value: "serif", label: "Young Serif" },
    ],
  },
  {
    id: "weight",
    label: "Graisse des titres",
    options: [
      { value: "leger", label: "Léger" },
      { value: "moyen", label: "Moyen" },
      { value: "gras", label: "Gras" },
    ],
  },
  {
    id: "italic",
    label: "Mot en italique",
    options: [
      { value: "garde", label: "Gardé" },
      { value: "orange", label: "Gardé, en orange" },
      { value: "non", label: "Enlevé" },
    ],
  },
  {
    id: "logo",
    label: "Logo du haut",
    options: [
      { value: "grand", label: "176 px qui flotte" },
      { value: "moyen", label: "128 px" },
      { value: "mascotte", label: "Mascotte détourée" },
    ],
  },
  {
    id: "day",
    label: "Une journée",
    options: [
      { value: "fixe", label: "iPhone fixe, heure géante" },
      { value: "cartes", label: "Cartes qui défilent" },
    ],
  },
  {
    id: "hour",
    label: "Heure géante",
    options: [
      { value: "clair", label: "Orange très clair" },
      { value: "contour", label: "Contour orange" },
      { value: "plein", label: "Orange plein" },
    ],
  },
  {
    id: "hermes",
    label: "Chapitre Hermes",
    options: [
      { value: "bloc", label: "Bloc orange plein" },
      { value: "cartes", label: "Blanc, cartes orange" },
      { value: "filets", label: "Blanc, filets noirs" },
    ],
  },
  {
    id: "offers",
    label: "Les deux offres",
    options: [
      { value: "mixte", label: "Blanc bordé et orange" },
      { value: "orange", label: "Deux blocs orange" },
      { value: "colonnes", label: "Deux colonnes" },
    ],
  },
  {
    id: "leave",
    label: "Ce qui sort",
    options: [
      { value: "orange", label: "Orange plein" },
      { value: "filet", label: "Blanc, filet noir" },
    ],
  },
  {
    id: "end",
    label: "Fin de page",
    options: [
      { value: "mascotte", label: "Bloc orange et mascotte" },
      { value: "bloc", label: "Bloc orange sans mascotte" },
      { value: "blanc", label: "Blanc, comme avant" },
    ],
  },
  {
    id: "tiles",
    label: "Quatre tuiles",
    options: [
      { value: "blanc", label: "Blanches" },
      { value: "pale", label: "Orange très clair" },
      { value: "orange", label: "Une tuile orange" },
    ],
  },
  {
    id: "round",
    label: "Coins",
    options: [
      { value: "doux", label: "Arrondis" },
      { value: "net", label: "Peu arrondis" },
      { value: "carre", label: "Carrés" },
    ],
  },
  {
    id: "line",
    label: "Traits",
    options: [
      { value: "gris", label: "Gris fin" },
      { value: "noir", label: "Noir fin" },
      { value: "epais", label: "Noir épais" },
    ],
  },
  {
    id: "text",
    label: "Petit texte",
    options: [
      { value: "noir", label: "Noir" },
      { value: "gris", label: "Gris" },
    ],
  },
  {
    id: "size",
    label: "Taille des titres",
    options: [
      { value: "normal", label: "Normale" },
      { value: "grand", label: "Très grande" },
    ],
  },
];

export const presets = [
  { id: "draft-1", name: "1. Tes choix", state: {} },
  {
    id: "draft-2",
    name: "2. Orange à fond",
    state: {
      promo: "orange",
      herobg: "orange",
      weight: "gras",
      italic: "non",
      logo: "mascotte",
      hour: "plein",
      tiles: "orange",
    },
  },
  {
    id: "draft-3",
    name: "3. Très net",
    state: {
      weight: "moyen",
      italic: "orange",
      logo: "moyen",
      hour: "contour",
      hermes: "filets",
      offers: "colonnes",
      leave: "filet",
      end: "blanc",
      round: "carre",
      line: "noir",
    },
  },
  {
    id: "draft-4",
    name: "4. Young Serif",
    state: {
      type: "serif",
      italic: "non",
      hermes: "cartes",
      offers: "orange",
      tiles: "pale",
    },
  },
  {
    id: "draft-5",
    name: "5. Ta page, retouchée",
    state: { type: "clash", italic: "orange", day: "cartes", end: "blanc" },
  },
  {
    id: "draft-6",
    name: "6. Côte à côte",
    state: { hero: "cote", logo: "moyen", round: "net", line: "noir" },
  },
  {
    id: "draft-7",
    name: "7. Titre géant",
    state: {
      hero: "geant",
      logo: "moyen",
      size: "grand",
      weight: "gras",
      italic: "orange",
      hour: "plein",
    },
  },
  {
    id: "draft-8",
    name: "8. Carré épais",
    state: {
      round: "carre",
      line: "epais",
      weight: "gras",
      hermes: "cartes",
      tiles: "orange",
      hour: "contour",
    },
  },
  {
    id: "draft-9",
    name: "9. Lumineux",
    state: {
      promo: "orange",
      orange: "clair",
      tiles: "pale",
      italic: "orange",
      weight: "moyen",
      size: "grand",
    },
  },
  {
    id: "draft-10",
    name: "10. Orange du logo",
    state: { orange: "logo", type: "clash", weight: "moyen", tiles: "pale" },
  },
  {
    id: "draft-11",
    name: "11. Mascotte partout",
    state: {
      hero: "cote",
      herobg: "orange",
      logo: "mascotte",
      tiles: "orange",
      weight: "gras",
      italic: "non",
    },
  },
  {
    id: "draft-12",
    name: "12. Blanc sobre",
    state: {
      hermes: "filets",
      leave: "filet",
      end: "bloc",
      italic: "garde",
      logo: "moyen",
      line: "noir",
    },
  },
];
