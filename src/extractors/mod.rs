pub mod sga;
pub mod gene;

use crate::types::{Config, ExtractorArgs};
use crate::error::Result;
use csv::{ReaderBuilder, StringRecord};
use std::fs::File;
use std::path::Path;

// Shared utility functions for extractors
pub(crate) fn create_csv_reader(path: &Path, delimiter: char) -> Result<csv::Reader<File>> {
    let reader = ReaderBuilder::new()
        .delimiter(delimiter as u8)
        .flexible(true)
        .from_path(path)?;
    Ok(reader)
}

pub(crate) fn create_csv_writer(path: &Path) -> Result<csv::Writer<File>> {
    let writer = csv::Writer::new(File::create(path)?);
    Ok(writer)
}

// Headers for different file types
pub(crate) const GWAS_HEADERS: [&str; 37] = [
    "MarkerID", "pval", "Phenotype", "Study", "PMID", "StudyGenomeBuild", "dbsnp.rsid",
    "dbsnp.dbsnp_build", "dbsnp.alleles.allele", "dbsnp.chrom", "dbsnp.hg19.start",
    "dbsnp.hg19.end", "dbsnp.vartype", "gnomad_genome.af.af", "gnomad_genome.af.af_afr",
    "gnomad_genome.af.af_amr", "gnomad_genome.af.af_asj", "gnomad_genome.af.af_eas",
    "gnomad_genome.af.af_fin", "gnomad_genome.af.af_nfe", "gnomad_genome.af.af_oth",
    "snpeff.ann.gene_id", "snpeff.ann.effect", "snpeff.ann.putative_impact",
    "snpeff.ann.feature_id", "snpeff.ann.hgvs_p", "snpeff.ann.protein.length",
    "dbnsfp.chrom", "dbnsfp.hg18.start", "dbnsfp.hg18.end", "dbnsfp.hg19.start",
    "dbnsfp.hg19.end", "dbnsfp.hg38.start", "dbnsfp.hg38.end", "dbnsfp.ensembl.proteinid",
    "dbnsfp.ensembl.transcriptid", "clinvar.rcv.clinical_significance"
];

pub(crate) const TRAIT_HEADERS: [&str; 37] = GWAS_HEADERS; // Using same headers for now