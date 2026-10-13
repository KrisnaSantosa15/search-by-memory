from fetch_public_corpus import wikitext_to_text


def test_wikitext_keeps_prose_and_link_text_but_drops_infoboxes():
    wikitext = (
        "{{Infobox makanan\n| nama = Rendang\n| asal = Sumatera Barat\n}}\n"
        "'''Rendang''' adalah masakan [[Minangkabau|Minang]] dari [[Sumatera Barat]].\n"
    )

    assert wikitext_to_text(wikitext) == "Rendang adalah masakan Minang dari Sumatera Barat.\n"
