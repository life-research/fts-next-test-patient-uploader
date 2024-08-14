# FTSnext Test Patient Uploader

The FTSnext Test Patient Uploader is a tool to populate the clinical domain of the FTSnext setup with test patients.

Therefore, it uses the output of Synthea-MII-KDS to generate and upload consent to gICS and upload FHIR resources to CD-HDS of fts-next.

## Usage

``` sh
upload_test_patients --data-dir PATH/TO/Synthea-MII-KDS/output/1000_Patients --docker-compose PATH/TO/fts-next/.github/test/compose.yaml --consent-template PATH/TO/fts-next/.github/scripts/consent.tmpl.json
```
or 
``` sh
cargo run -- --data-dir PATH/TO/Synthea-MII-KDS/output/1000_Patients --docker-compose PATH/TO/fts-next/.github/test/compose.yaml --consent-template PATH/TO/fts-next/.github/scripts/consent.tmpl.json
```

### Options
-n <N> Upload N patients

--ids <comma separated list of IDs> upload specific IDs
