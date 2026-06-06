use crate::{
    DecryptOptions, EncryptOptions, OperationResult, TempDecryptedFile, decrypt_file, encrypt_file,
    is_encrypted_path, is_metadata_path, to_decrypted_path, to_encrypted_path,
};
use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::{IsTerminal, Write, stdin};
use std::path::PathBuf;

pub struct EditOptions {
    pub path: PathBuf,
    pub decrypt_options: DecryptOptions,
    pub encrypt_options: EncryptOptions,
    pub clean: bool,
}

pub fn edit(opts: EditOptions) -> Result<Box<dyn Iterator<Item = Result<OperationResult>>>> {
    let path = &opts.path;
    if is_metadata_path(path) {
        return Err(anyhow!("{}: could not edit metadata file", path.display()));
    }

    let is_target_encrypted = is_encrypted_path(path);

    let (decrypted_path, encrypted_path) = if is_target_encrypted {
        let dec = to_decrypted_path(path)
            .ok_or_else(|| anyhow!("{}: invalid encrypted path", path.display()))?;
        (dec, path.clone())
    } else {
        (path.clone(), to_encrypted_path(path))
    };

    let temp_file = if is_target_encrypted {
        Some(TempDecryptedFile::new(decrypted_path.clone(), opts.clean))
    } else {
        None
    };

    let mut results: Vec<Result<OperationResult>> = vec![];

    let status1 = if !stdin().is_terminal() {
        let mut outfile = File::create(&decrypted_path)?;
        let mut writer = std::io::BufWriter::new(&mut outfile);
        let infile = stdin().lock();
        let mut reader = std::io::BufReader::new(infile);
        std::io::copy(&mut reader, &mut writer)?;
        writer.flush()?;
        Ok(())
    } else {
        if is_target_encrypted
            && let Some(res) = decrypt_file(&encrypted_path, &opts.decrypt_options)?
        {
            results.push(Ok(res));
        }

        ::edit::edit_file(&decrypted_path).map_err(|e| anyhow!(e))
    };

    let status2 = {
        let enc_status = encrypt_file(&decrypted_path, &opts.encrypt_options, None);
        match enc_status {
            Ok(Some(res)) => {
                results.push(Ok(res));
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    };

    if let Some(mut tf) = temp_file {
        if opts.clean {
            match tf.cleanup(false) {
                Ok(Some(res)) => results.push(Ok(res)),
                Ok(None) => {}
                Err(e) => results.push(Err(e)),
            }
        } else {
            tf.disarm();
        }
    }

    // Now fail if status1 or status2 failed, but collect results first
    status1?;
    status2?;

    Ok(Box::new(results.into_iter()))
}
