#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum Variable {
    /// abstract of the item (e.g. the abstract of a journal article)
    Abstract,
    /// reader’s notes about the item content
    Annote,
    /// archive storing the item
    Archive,
    /// storage location within an archive (e.g. a box and folder number)
    /// technically the spec says use an underscore, but that's probably a typo.
    #[strum(
        serialize = "archive_location",
        serialize = "archive-location"
    )]
    ArchiveLocation,
    /// geographic location of the archive,
    ArchivePlace,
    /// issuing or judicial authority (e.g. “USPTO” for a patent, “Fairfax Circuit Court” for a legal case)
    #[strum(props(csl101 = "1", cslM = "0"))]
    Authority,
    /// active={true} call number (to locate the item in a library)
    CallNumber,
    /// label identifying the item in in-text citations of label styles (e.g. “Ferr78”). May be assigned by the CSL processor based on item metadata.
    CitationLabel,
    /// index (starting at 1) of the cited reference in the bibliography (generated by the CSL processor)
    CitationNumber,
    /// title of the collection holding the item (e.g. the series title for a book)
    CollectionTitle,
    /// title of the container holding the item (e.g. the book title for a book chapter, the journal title for a journal article)
    ContainerTitle,
    /// short/abbreviated form of “container-title” (also accessible through the “short” form of the “container-title” variable)
    ContainerTitleShort,
    /// physical (e.g. size) or temporal (e.g. running time) dimensions of the item
    Dimensions,
    /// Digital Object Identifier (e.g. “10.1128/AEM.02591-07”)
    #[strum(serialize = "DOI")]
    DOI,
    /// name of the related event (e.g. the conference name when citing a conference paper)
    Event,
    /// geographic location of the related event (e.g. “Amsterdam, the Netherlands”)
    EventPlace,
    /// number of a preceding note containing the first reference to the item. Assigned by the CSL processor. The variable holds no value for non-note-based styles, or when the item hasn’t been cited in any preceding notes.
    FirstReferenceNoteNumber,
    /// class, type or genre of the item (e.g. “adventure” for an adventure movie, “PhD dissertation” for a PhD thesis)
    Genre,
    /// International Standard Book Number
    ISBN,
    /// International Standard Serial Number
    ISSN,
    /// geographic scope of relevance (e.g. “US” for a US patent)
    Jurisdiction,
    /// keyword(s) or tag(s) attached to the item
    Keyword,
    /// a cite-specific pinpointer within the item (e.g. a page number within a book, or a volume in a multi-volume work). Must be accompanied in the input data by a label indicating the locator type (see the Locators term list), which determines which term is rendered by cs:label when the “locator” variable is selected.
    #[strum(props(csl101 = "1", cslM = "0"))]
    Locator,
    /// medium description (e.g. “CD”, “DVD”, etc.)
    Medium,
    /// (short) inline note giving additional item details (e.g. a concise summary or commentary)
    Note,
    /// original publisher, for items that have been republished by a different publisher
    OriginalPublisher,
    /// geographic location of the original publisher (e.g. “London, UK”)
    OriginalPublisherPlace,
    /// title of the original version (e.g. “Война и мир”, the untranslated Russian title of “War and Peace”)
    OriginalTitle,
    /// range of pages the item (e.g. a journal article) covers in a container (e.g. a journal issue)
    #[strum(props(csl101 = "1", cslM = "0"))]
    Page,
    /// first page of the range of pages the item (e.g. a journal article) covers in a container (e.g. a journal issue)
    #[strum(props(csl101 = "1", cslM = "0"))]
    PageFirst,
    /// PubMed Central reference number
    #[strum(serialize = "PMCID")]
    PMCID,
    /// PubMed reference number
    #[strum(serialize = "PMID")]
    PMID,
    /// publisher
    Publisher,
    /// geographic location of the publisher
    PublisherPlace,
    /// resources related to the procedural history of a legal case
    References,
    /// title of the item reviewed by the current item
    ReviewedTitle,
    /// scale of e.g. a map
    Scale,
    /// container section holding the item (e.g. “politics” for a newspaper article)
    Section,
    /// from whence the item originates (e.g. a library catalog or database)
    Source,
    /// (publication) status of the item (e.g. “forthcoming”)
    Status,
    /// primary title of the item
    Title,
    /// short/abbreviated form of “title” (also accessible through the “short” form of the “title” variable)
    TitleShort,
    ///  URL (e.g. “http://aem.asm.org/cgi/content/full/74/9/2766”)
    #[strum(serialize = "URL")]
    URL,
    /// version of the item (e.g. “2.0.9” for a software program)
    Version,
    /// disambiguating year suffix in author-date styles (e.g. “a” in “Doe, 1999a”)
    YearSuffix,

    // CSL-M Additions
    #[strum(props(csl101 = "0", cslM = "1"))]
    Hereinafter,
    #[strum(props(csl101 = "0", cslM = "1"))]
    AvailableDate,
    #[strum(props(csl101 = "0", cslM = "1"))]
    Dummy,
    #[strum(props(csl101 = "0", cslM = "1"))]
    LocatorExtra,
    #[strum(props(csl101 = "0", cslM = "1"))]
    VolumeTitle,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum NumberVariable {
    ChapterNumber,
    CollectionNumber,
    Edition,
    Issue,
    Number,
    NumberOfPages,
    NumberOfVolumes,
    Volume,
    #[strum(props(csl101 = "0", cslM = "1"))]
    Locator,
    #[strum(props(csl101 = "0", cslM = "1"))]
    Page,
    #[strum(props(csl101 = "0", cslM = "1"))]
    PageFirst,

    #[strum(props(csl101 = "0", cslM = "1"))]
    PublicationNumber,

    #[strum(props(csl101 = "0", cslM = "1"))]
    Supplement,

    #[strum(props(csl101 = "0", cslM = "1"))]
    Authority,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum NameVariable {
    /// author
    Author,
    /// editor of the collection holding the item (e.g. the series editor for a book)
    CollectionEditor,
    /// composer (e.g. of a musical score)
    Composer,
    /// author of the container holding the item (e.g. the book author for a book chapter)
    ContainerAuthor,
    /// director (e.g. of a film)
    Director,
    /// editor
    Editor,
    /// managing editor (“Directeur de la Publication” in French)
    EditorialDirector,
    /// illustrator (e.g. of a children’s book)
    Illustrator,
    /// interviewer (e.g. of an interview)
    Interviewer,
    /// ?
    OriginalAuthor,
    /// recipient (e.g. of a letter)
    Recipient,
    /// author of the item reviewed by the current item
    ReviewedAuthor,
    /// translator
    Translator,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum DateVariable {
    /// date the item has been accessed
    Accessed,
    /// ?
    Container,
    /// date the related event took place
    EventDate,
    /// date the item was issued/published
    Issued,
    /// (issue) date of the original version
    OriginalDate,
    /// date the item (e.g. a manuscript) has been submitted for publication
    Submitted,
    #[strum(props(csl101 = "0", cslM = "1"))]
    LocatorDate,
    #[strum(props(csl101 = "0", cslM = "1"))]
    PublicationDate,
}
