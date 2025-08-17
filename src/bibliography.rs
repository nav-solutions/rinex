#![allow(dead_code)]

/// Important articles and references that proved useful when designing this library
pub enum Bibliography {
    /// RINEX V2.11 specifications by IGS.
    /// [DOI](https://files.igs.org/pub/data/format/rinex211.pdf).
    RINEX211,

    /// RINEX V3 specifications by IGS.
    /// [DOI](https://files.igs.org/pub/data/format/rinex300.pdf).
    RINEX3,

    /// RINEX V4 specifications by IGS.
    /// [DOI](https://files.igs.org/pub/data/format/rinex_4.00.pdf).
    RINEX4,

    /// J. Lesouple, 2019: *Estimation Parcimonieuse de Biais Multitrajets pour Systemes GNSS*.
    /// Pseudo range calculation method on page 50.
    /// Kepler Solver on page 159.
    /// Elevation and Azimuth angles determination, page 160.
    /// [DOI](http://perso.recherche.enac.fr/~julien.lesouple/fr/publication/thesis/THESIS.pdf?fbclid=IwAR3WlHm0eP7ygRzywbL07Ig-JawvsdCEdvz1umJJaRRXVO265J9cp931YyI)
    JLe19,

    /// ESA NAVIPedia: *Combining Pairs of signals and clock definitions*.
    /// [DOI](https://gssc.esa.int/navipedia/index.php/Combining_pairs_of_signals_and_clock_definition)
    ESAGnssCombination,

    /// ASCE Appendix 3: *Calculation of Satellite Position from Ephemeris Data*.
    /// [DOI](https://ascelibrary.org/doi/pdf/10.1061/9780784411506.ap03).
    AsceAppendix3,

    /// ESA GNSS Data Processing Book Vol. I.
    /// [DOI](https://gssc.esa.int/navipedia/GNSS_Book/ESA_GNSS-Book_TM-23_Vol_I.pdf).
    ESABookVol1,

    /// ESA GNSS Data Processing Book Vol. II.
    /// [DOI](https://gssc.esa.int/navipedia/GNSS_Book/ESA_GNSS-Book_TM-23_Vol_II.pdf).
    ESABookVol2,

    /// E. Schönemann, M. Becker, T. Springer, 2011:
    /// *A new Approach for GNSS Analysis in a Multi-GNSS and Multi-Signal Environment*.
    /// [DOI](https://www.degruyter.com/document/doi/10.2478/v10156-010-0023-2/pdf).
    GeoScienceJournal1,

    /// V. Pinazo Garcia, N. Woodhouse:
    /// *Multipath Analysis Using Code-Minus-Carrier technique in
    /// GNSS antennas*.
    /// [DOI](https://cdn.taoglas.com/wp-content/uploads/pdf/Multipath-Analysis-Using-Code-Minus-Carrier-Technique-in-GNSS-Antennas-_WhitePaper_VP__Final-1.pdf).
    MpTaoglas,

    /// BeiDou Navigation Satellite System Signal In Space Interface Control Document(BDS-3)
    /// [B1I](http://www.beidou.gov.cn/xt/gfxz/201902/P020190227593621142475.pdf)
    /// [B1c](http://www.beidou.gov.cn/xt/gfxz/201712/P020171226741342013031.pdf)
    /// [B2a](http://www.beidou.gov.cn/xt/gfxz/201712/P020171226742357364174.pdf)
    /// [B2b](http://www.beidou.gov.cn/xt/gfxz/202008/P020230516558683155109.pdf)
    /// [B3I](http://www.beidou.gov.cn/xt/gfxz/201802/P020180209623601401189.pdf)
    BeiDouICD,
}
