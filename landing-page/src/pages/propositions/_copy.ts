export const moments = [
  {
    id: "matin",
    hour: "9:40",
    label: "En sortant du rendez-vous",
    said: "Rappelle-moi d'envoyer la proposition jeudi à 9 h.",
    does: "C'est écrit, titré et rangé. Sur iPhone, le rappel arrive dans ton agenda d'un toucher.",
    clip: "dictate",
  },
  {
    id: "midi",
    hour: "11:00",
    label: "Pendant la réunion",
    said: "Bon, où on en est sur le lancement ?",
    does: "La réunion se lit comme un document : chapitres, décisions, actions, avec le moment où chacune a été dite.",
    clip: "meeting",
  },
  {
    id: "aprem",
    hour: "14:15",
    label: "Avant d'appeler le client",
    said: "Que reste-t-il avant le lancement ?",
    does: "Tes notes répondent et citent leurs sources. Un toucher ouvre la note.",
    clip: "answer",
  },
  {
    id: "soir",
    hour: "19:00",
    label: "Pendant que tu rentres",
    said: "Fais-en mon plan de lancement.",
    does: "Une photo du tableau à ton propre Hermes, et le plan arrive. Tu suis son travail depuis ton iPhone.",
    clip: "agent",
  },
];

export const offers = {
  title: "Toi, ton équipe, ton agent. Un seul espace.",
  text: "Un espace réunit des notes, les personnes que tu invites et ton Hermes, avec un accès limité et révocable.",
  solo: {
    name: "Tu testes seul",
    text: "L'app, tes clés, ton rythme. Sans compte, et le code est ouvert.",
    stub: "Gratuit pour l'instant",
    cta: "Télécharger",
  },
  guided: {
    name: "On t'accompagne",
    text: "Mirko et son agent rejoignent ton espace pour voir ce que tu peux déléguer.",
    stub: "30 min offertes",
    cta: "Prendre rendez-vous",
  },
};

export const trust = {
  stay: "Sur tes appareils : tes notes, tes audios, ta recherche et la transcription Whisper.",
  leave:
    "Quand tu t'en sers : le chat et les résumés partent chez OpenAI ou Anthropic, la transcription cloud chez Soniox, le partage par le serveur FlowFlow.",
  cost: "Une heure de réunion transcrite : environ 0,10 $ chez Soniox, 0 € avec Whisper sur ton appareil.",
};
