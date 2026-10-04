//! Journal pages left by apprentices who went down before you. Read in a
//! parchment popup; which pages you've read is saved across runs.

pub struct Page {
    pub author: &'static str,
    pub text: &'static str,
}

pub static JOURNALS: [Page; 30] = [
    Page {
        author: "Apprentice Mirel",
        text: "The Master says the heart of the world sings to those who are ready. I hear nothing yet. Only the dripping.",
    },
    Page {
        author: "Apprentice Mirel",
        text: "Third night. Something answered when I called the light. Not an echo. An echo does not breathe.",
    },
    Page {
        author: "Unsigned",
        text: "If you are reading this, put out your light and listen. It finds the light first.",
    },
    Page {
        author: "Apprentice Tobin",
        text: "Bound to fire and water both. Steam everywhere. I am never cold, but I can no longer see my hands.",
    },
    Page {
        author: "Apprentice Tobin",
        text: "The altar asked me to choose. I chose quickly. I should have chosen well.",
    },
    Page {
        author: "Apprentice Hesk",
        text: "The stone here remembers being sky. When I dig, it sighs.",
    },
    Page {
        author: "Apprentice Hesk",
        text: "Gravel ceilings. Do not run. Do not shout. Do not dig beneath them unless you mean to.",
    },
    Page {
        author: "Unsigned",
        text: "The halls below are drowned. Somebody built them. Somebody wanted them flooded.",
    },
    Page {
        author: "Apprentice Ruu",
        text: "Fungus everywhere, glowing like a festival. It burns like one too. Lost my eyebrows. Kept my life.",
    },
    Page {
        author: "Apprentice Ruu",
        text: "The puppets were apprentices once. Their robes are still under the mushrooms.",
    },
    Page {
        author: "Apprentice Cael",
        text: "There is a tall thing that walks without a face. It does not hurry. It does not need to.",
    },
    Page {
        author: "Apprentice Cael",
        text: "It will not cross water. I sit in the pool and it stands at the edge, waiting. I am so tired.",
    },
    Page {
        author: "Unsigned",
        text: "Buried it under the gravel. It stopped. Then it sank, like a stone into a well. I hope it stays down.",
    },
    Page {
        author: "Apprentice Ilse",
        text: "Lightning through the water is beautiful until you are standing in the water.",
    },
    Page {
        author: "Apprentice Ilse",
        text: "Ice over lava makes black glass. Black glass makes bridges. Write that down. I am writing it down.",
    },
    Page {
        author: "Apprentice Varo",
        text: "Acid eats all but the black glass and the crystal. Remember which is which before you throw the flask.",
    },
    Page {
        author: "Apprentice Varo",
        text: "A chest opened its mouth at me. I am done opening chests. I am done with a great many things.",
    },
    Page {
        author: "Master Orren",
        text: "Every apprentice believes the descent is a test of power. It is a test of attention.",
    },
    Page {
        author: "Master Orren",
        text: "Two schools, apprentice. One is a tool. Two is a language. Learn to speak.",
    },
    Page {
        author: "Unsigned",
        text: "The worm hears the digging. Dig and it comes. Stop and it forgets. I have stopped.",
    },
    Page {
        author: "Apprentice Nimh",
        text: "The deeper I go the warmer my blood. The Sanctum is a forge, and we are the iron.",
    },
    Page {
        author: "Apprentice Nimh",
        text: "Molten metal holds the lightning like a jar holds water. Do not touch the jar.",
    },
    Page {
        author: "Apprentice Bel",
        text: "Counted the remains on the way down. Eleven. I stopped counting. It felt like rudeness.",
    },
    Page {
        author: "Apprentice Bel",
        text: "Their shards still glow. Even the dead pay their way.",
    },
    Page {
        author: "Unsigned",
        text: "Down here my light orb flickers when no wind blows. Something drinks from it.",
    },
    Page {
        author: "Apprentice Sorrel",
        text: "The heart is close. I can feel it in my teeth. Gold and white and patient.",
    },
    Page {
        author: "Apprentice Sorrel",
        text: "Touched the shell of the heart chamber. Warm. It wanted me to find the door.",
    },
    Page {
        author: "Master Orren",
        text: "Those who complete the rite return changed. Those who do not, return as well, after a fashion.",
    },
    Page {
        author: "Unsigned",
        text: "We were never meant to all come back. The heart only needs one of us to listen.",
    },
    Page {
        author: "Apprentice Mirel",
        text: "I hear it now. It is singing my name. It has been singing it the whole time.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_short_enough_for_the_popup() {
        assert!(JOURNALS.len() >= 30);
        for p in &JOURNALS {
            assert!(p.text.len() < 200, "{}", p.text);
        }
    }
}
