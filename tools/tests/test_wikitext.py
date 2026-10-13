from fetch_public_corpus import wikitext_to_text


def test_wikitext_keeps_prose_and_link_text_but_drops_infoboxes():
    wikitext = (
        "{{Infobox makanan\n| nama = Rendang\n| asal = Sumatera Barat\n}}\n"
        "'''Rendang''' adalah masakan [[Minangkabau|Minang]] dari [[Sumatera Barat]].\n"
    )

    assert wikitext_to_text(wikitext) == "Rendang adalah masakan Minang dari Sumatera Barat.\n"


def test_wikitext_drops_references():
    wikitext = (
        'Rendang dinobatkan sebagai hidangan terlezat.<ref name="cnn">https://edition.cnn.com/travel</ref> '
        "Pada 2018 rendang menjadi hidangan nasional.<ref>{{cite web|url=https://kompas.com}}</ref><ref name=\"cnn\" />\n"
    )

    assert wikitext_to_text(wikitext) == (
        "Rendang dinobatkan sebagai hidangan terlezat. Pada 2018 rendang menjadi hidangan nasional.\n"
    )
