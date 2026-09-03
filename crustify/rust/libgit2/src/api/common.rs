//! Safe wrappers for libgit2 common APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_buildinfo_t
/// A checked kind of compile-time build information exposed by libgit2.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitBuildInfo {
    /// The CPU architecture for which libgit2 was built.
    Cpu = ffi::git_buildinfo_t_GIT_BUILDINFO_CPU,
    /// The source commit from which libgit2 was built.
    Commit = ffi::git_buildinfo_t_GIT_BUILDINFO_COMMIT,
}

/// A raw build-information kind not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitBuildInfo(ffi::git_buildinfo_t);

impl InvalidGitBuildInfo {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_buildinfo_t {
        self.0
    }
}

impl From<GitBuildInfo> for ffi::git_buildinfo_t {
    fn from(info: GitBuildInfo) -> Self {
        info as Self
    }
}

impl TryFrom<ffi::git_buildinfo_t> for GitBuildInfo {
    type Error = InvalidGitBuildInfo;

    fn try_from(info: ffi::git_buildinfo_t) -> Result<Self, Self::Error> {
        match info {
            ffi::git_buildinfo_t_GIT_BUILDINFO_CPU => Ok(Self::Cpu),
            ffi::git_buildinfo_t_GIT_BUILDINFO_COMMIT => Ok(Self::Commit),
            value => Err(InvalidGitBuildInfo(value)),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_build_information_kinds_round_trip() {
        for info in [GitBuildInfo::Cpu, GitBuildInfo::Commit] {
            let raw = ffi::git_buildinfo_t::from(info);
            assert_eq!(GitBuildInfo::try_from(raw), Ok(info));
        }
    }

    #[test]
    fn unknown_build_information_kind_is_rejected() {
        let raw = ffi::git_buildinfo_t_GIT_BUILDINFO_COMMIT + 1;
        assert_eq!(GitBuildInfo::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn build_information_kind_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<GitBuildInfo>(), size_of::<ffi::git_buildinfo_t>());
        assert_eq!(
            align_of::<GitBuildInfo>(),
            align_of::<ffi::git_buildinfo_t>()
        );
    }
}

/// Wraps: git_feature_t
/// A set of compile-time features supported by libgit2.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitFeatureFlags(ffi::git_feature_t);

impl GitFeatureFlags {
    /// No optional or built-in feature bits.
    pub const NONE: Self = Self(0);
    /// Thread-safe operation.
    pub const THREADS: Self = Self(ffi::git_feature_t_GIT_FEATURE_THREADS);
    /// HTTPS remotes.
    pub const HTTPS: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTPS);
    /// SSH remotes.
    pub const SSH: Self = Self(ffi::git_feature_t_GIT_FEATURE_SSH);
    /// Sub-second index timestamp resolution.
    pub const NSEC: Self = Self(ffi::git_feature_t_GIT_FEATURE_NSEC);
    /// HTTP parsing support.
    pub const HTTP_PARSER: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTP_PARSER);
    /// Regular-expression support.
    pub const REGEX: Self = Self(ffi::git_feature_t_GIT_FEATURE_REGEX);
    /// Filename internationalization support.
    pub const I18N: Self = Self(ffi::git_feature_t_GIT_FEATURE_I18N);
    /// NTLM authentication.
    pub const AUTH_NTLM: Self = Self(ffi::git_feature_t_GIT_FEATURE_AUTH_NTLM);
    /// Negotiate (SPNEGO) authentication.
    pub const AUTH_NEGOTIATE: Self = Self(ffi::git_feature_t_GIT_FEATURE_AUTH_NEGOTIATE);
    /// Compression support.
    pub const COMPRESSION: Self = Self(ffi::git_feature_t_GIT_FEATURE_COMPRESSION);
    /// SHA-1 object support.
    pub const SHA1: Self = Self(ffi::git_feature_t_GIT_FEATURE_SHA1);
    /// SHA-256 object support.
    pub const SHA256: Self = Self(ffi::git_feature_t_GIT_FEATURE_SHA256);
    /// Plain HTTP remotes.
    pub const HTTP: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTP);
    /// Every feature bit published by these bindings.
    pub const ALL: Self = Self(
        Self::THREADS.0
            | Self::HTTPS.0
            | Self::SSH.0
            | Self::NSEC.0
            | Self::HTTP_PARSER.0
            | Self::REGEX.0
            | Self::I18N.0
            | Self::AUTH_NTLM.0
            | Self::AUTH_NEGOTIATE.0
            | Self::COMPRESSION.0
            | Self::SHA1.0
            | Self::SHA256.0
            | Self::HTTP.0,
    );

    /// Converts raw bits when every bit is published by these bindings.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_feature_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Retains all raw bits, including features added by a newer linked library.
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_feature_t) -> Self {
        Self(bits)
    }

    /// Returns the underlying libgit2 feature bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_feature_t {
        self.0
    }

    /// Returns whether no feature bit is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every feature in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any feature in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitFeatureFlags> for ffi::git_feature_t {
    fn from(flags: GitFeatureFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_feature_t> for GitFeatureFlags {
    type Error = ffi::git_feature_t;

    fn try_from(bits: ffi::git_feature_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitFeatureFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitFeatureFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitFeatureFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitFeatureFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitFeatureFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod feature_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn feature_sets_combine_and_validate() {
        let features = GitFeatureFlags::THREADS | GitFeatureFlags::HTTP;
        assert!(features.contains(GitFeatureFlags::THREADS));
        assert!(features.intersects(GitFeatureFlags::HTTP));
        assert_eq!(GitFeatureFlags::from_bits(features.bits()), Some(features));
        assert!(GitFeatureFlags::NONE.is_empty());

        let unknown = GitFeatureFlags::ALL.bits() << 1;
        assert_eq!(GitFeatureFlags::from_bits(unknown), None);
        assert_eq!(GitFeatureFlags::from_bits_retain(unknown).bits(), unknown);
    }

    #[test]
    fn feature_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFeatureFlags>(),
            size_of::<ffi::git_feature_t>()
        );
        assert_eq!(
            align_of::<GitFeatureFlags>(),
            align_of::<ffi::git_feature_t>()
        );
    }
}

/// Wraps: git_libgit2_opt_t
/// A checked selector for libgit2's process-global option API.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitLibgit2Option {
    /// Query the maximum memory-map window size.
    GetMwindowSize = ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_SIZE,
    /// Set the maximum memory-map window size.
    SetMwindowSize = ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_SIZE,
    /// Query the total mapped-memory limit.
    GetMwindowMappedLimit = ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_MAPPED_LIMIT,
    /// Set the total mapped-memory limit.
    SetMwindowMappedLimit = ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_MAPPED_LIMIT,
    /// Query a configuration search path.
    GetSearchPath = ffi::git_libgit2_opt_t_GIT_OPT_GET_SEARCH_PATH,
    /// Set a configuration search path.
    SetSearchPath = ffi::git_libgit2_opt_t_GIT_OPT_SET_SEARCH_PATH,
    /// Set the cache limit for one object kind.
    SetCacheObjectLimit = ffi::git_libgit2_opt_t_GIT_OPT_SET_CACHE_OBJECT_LIMIT,
    /// Set the total object-cache size.
    SetCacheMaxSize = ffi::git_libgit2_opt_t_GIT_OPT_SET_CACHE_MAX_SIZE,
    /// Enable or disable object caching.
    EnableCaching = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_CACHING,
    /// Query current and allowed cache memory.
    GetCachedMemory = ffi::git_libgit2_opt_t_GIT_OPT_GET_CACHED_MEMORY,
    /// Query the repository template path.
    GetTemplatePath = ffi::git_libgit2_opt_t_GIT_OPT_GET_TEMPLATE_PATH,
    /// Set the repository template path.
    SetTemplatePath = ffi::git_libgit2_opt_t_GIT_OPT_SET_TEMPLATE_PATH,
    /// Set the TLS certificate file and directory.
    SetSslCertLocations = ffi::git_libgit2_opt_t_GIT_OPT_SET_SSL_CERT_LOCATIONS,
    /// Set the HTTP user-agent string.
    SetUserAgent = ffi::git_libgit2_opt_t_GIT_OPT_SET_USER_AGENT,
    /// Enable or disable strict object creation.
    EnableStrictObjectCreation = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_OBJECT_CREATION,
    /// Enable or disable strict symbolic-reference creation.
    EnableStrictSymbolicRefCreation =
        ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_SYMBOLIC_REF_CREATION,
    /// Set the TLS cipher list.
    SetSslCiphers = ffi::git_libgit2_opt_t_GIT_OPT_SET_SSL_CIPHERS,
    /// Query the HTTP user-agent string.
    GetUserAgent = ffi::git_libgit2_opt_t_GIT_OPT_GET_USER_AGENT,
    /// Enable or disable offset deltas.
    EnableOfsDelta = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_OFS_DELTA,
    /// Enable or disable `fsync` for Git directories.
    EnableFsyncGitdir = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_FSYNC_GITDIR,
    /// Query the Windows file-sharing mode.
    GetWindowsSharemode = ffi::git_libgit2_opt_t_GIT_OPT_GET_WINDOWS_SHAREMODE,
    /// Set the Windows file-sharing mode.
    SetWindowsSharemode = ffi::git_libgit2_opt_t_GIT_OPT_SET_WINDOWS_SHAREMODE,
    /// Enable or disable strict object-hash verification.
    EnableStrictHashVerification = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_HASH_VERIFICATION,
    /// Install a process-global allocator.
    SetAllocator = ffi::git_libgit2_opt_t_GIT_OPT_SET_ALLOCATOR,
    /// Enable or disable unsaved-index safety checks.
    EnableUnsavedIndexSafety = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_UNSAVED_INDEX_SAFETY,
    /// Query the pack object-count limit.
    GetPackMaxObjects = ffi::git_libgit2_opt_t_GIT_OPT_GET_PACK_MAX_OBJECTS,
    /// Set the pack object-count limit.
    SetPackMaxObjects = ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECTS,
    /// Disable or enable pack keep-file checks.
    DisablePackKeepFileChecks = ffi::git_libgit2_opt_t_GIT_OPT_DISABLE_PACK_KEEP_FILE_CHECKS,
    /// Enable or disable HTTP `Expect: 100-continue`.
    EnableHttpExpectContinue = ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_HTTP_EXPECT_CONTINUE,
    /// Query the mapped-file count limit.
    GetMwindowFileLimit = ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_FILE_LIMIT,
    /// Set the mapped-file count limit.
    SetMwindowFileLimit = ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_FILE_LIMIT,
    /// Set the packed ODB backend priority.
    SetOdbPackedPriority = ffi::git_libgit2_opt_t_GIT_OPT_SET_ODB_PACKED_PRIORITY,
    /// Set the loose ODB backend priority.
    SetOdbLoosePriority = ffi::git_libgit2_opt_t_GIT_OPT_SET_ODB_LOOSE_PRIORITY,
    /// Query the enabled index extensions.
    GetExtensions = ffi::git_libgit2_opt_t_GIT_OPT_GET_EXTENSIONS,
    /// Set the enabled index extensions.
    SetExtensions = ffi::git_libgit2_opt_t_GIT_OPT_SET_EXTENSIONS,
    /// Query repository-owner validation.
    GetOwnerValidation = ffi::git_libgit2_opt_t_GIT_OPT_GET_OWNER_VALIDATION,
    /// Set repository-owner validation.
    SetOwnerValidation = ffi::git_libgit2_opt_t_GIT_OPT_SET_OWNER_VALIDATION,
    /// Query the process home directory.
    GetHomedir = ffi::git_libgit2_opt_t_GIT_OPT_GET_HOMEDIR,
    /// Set the process home directory.
    SetHomedir = ffi::git_libgit2_opt_t_GIT_OPT_SET_HOMEDIR,
    /// Set the server connection timeout.
    SetServerConnectTimeout = ffi::git_libgit2_opt_t_GIT_OPT_SET_SERVER_CONNECT_TIMEOUT,
    /// Query the server connection timeout.
    GetServerConnectTimeout = ffi::git_libgit2_opt_t_GIT_OPT_GET_SERVER_CONNECT_TIMEOUT,
    /// Set the server operation timeout.
    SetServerTimeout = ffi::git_libgit2_opt_t_GIT_OPT_SET_SERVER_TIMEOUT,
    /// Query the server operation timeout.
    GetServerTimeout = ffi::git_libgit2_opt_t_GIT_OPT_GET_SERVER_TIMEOUT,
    /// Set the product portion of the user-agent string.
    SetUserAgentProduct = ffi::git_libgit2_opt_t_GIT_OPT_SET_USER_AGENT_PRODUCT,
    /// Query the product portion of the user-agent string.
    GetUserAgentProduct = ffi::git_libgit2_opt_t_GIT_OPT_GET_USER_AGENT_PRODUCT,
    /// Add an in-memory X.509 certificate.
    AddSslX509Cert = ffi::git_libgit2_opt_t_GIT_OPT_ADD_SSL_X509_CERT,
    /// Query the maximum size of one packed object.
    GetPackMaxObjectSize = ffi::git_libgit2_opt_t_GIT_OPT_GET_PACK_MAX_OBJECT_SIZE,
    /// Set the maximum size of one packed object.
    SetPackMaxObjectSize = ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECT_SIZE,
}

/// A raw global-option selector not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitLibgit2Option(ffi::git_libgit2_opt_t);

impl InvalidGitLibgit2Option {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_libgit2_opt_t {
        self.0
    }
}

impl From<GitLibgit2Option> for ffi::git_libgit2_opt_t {
    fn from(option: GitLibgit2Option) -> Self {
        option as Self
    }
}

impl TryFrom<ffi::git_libgit2_opt_t> for GitLibgit2Option {
    type Error = InvalidGitLibgit2Option;

    fn try_from(option: ffi::git_libgit2_opt_t) -> Result<Self, Self::Error> {
        match option {
            ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_SIZE => Ok(Self::GetMwindowSize),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_SIZE => Ok(Self::SetMwindowSize),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_MAPPED_LIMIT => {
                Ok(Self::GetMwindowMappedLimit)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_MAPPED_LIMIT => {
                Ok(Self::SetMwindowMappedLimit)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_GET_SEARCH_PATH => Ok(Self::GetSearchPath),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_SEARCH_PATH => Ok(Self::SetSearchPath),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_CACHE_OBJECT_LIMIT => Ok(Self::SetCacheObjectLimit),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_CACHE_MAX_SIZE => Ok(Self::SetCacheMaxSize),
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_CACHING => Ok(Self::EnableCaching),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_CACHED_MEMORY => Ok(Self::GetCachedMemory),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_TEMPLATE_PATH => Ok(Self::GetTemplatePath),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_TEMPLATE_PATH => Ok(Self::SetTemplatePath),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_SSL_CERT_LOCATIONS => Ok(Self::SetSslCertLocations),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_USER_AGENT => Ok(Self::SetUserAgent),
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_OBJECT_CREATION => {
                Ok(Self::EnableStrictObjectCreation)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_SYMBOLIC_REF_CREATION => {
                Ok(Self::EnableStrictSymbolicRefCreation)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_SET_SSL_CIPHERS => Ok(Self::SetSslCiphers),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_USER_AGENT => Ok(Self::GetUserAgent),
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_OFS_DELTA => Ok(Self::EnableOfsDelta),
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_FSYNC_GITDIR => Ok(Self::EnableFsyncGitdir),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_WINDOWS_SHAREMODE => Ok(Self::GetWindowsSharemode),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_WINDOWS_SHAREMODE => Ok(Self::SetWindowsSharemode),
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_HASH_VERIFICATION => {
                Ok(Self::EnableStrictHashVerification)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_SET_ALLOCATOR => Ok(Self::SetAllocator),
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_UNSAVED_INDEX_SAFETY => {
                Ok(Self::EnableUnsavedIndexSafety)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_GET_PACK_MAX_OBJECTS => Ok(Self::GetPackMaxObjects),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECTS => Ok(Self::SetPackMaxObjects),
            ffi::git_libgit2_opt_t_GIT_OPT_DISABLE_PACK_KEEP_FILE_CHECKS => {
                Ok(Self::DisablePackKeepFileChecks)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_HTTP_EXPECT_CONTINUE => {
                Ok(Self::EnableHttpExpectContinue)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_FILE_LIMIT => Ok(Self::GetMwindowFileLimit),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_FILE_LIMIT => Ok(Self::SetMwindowFileLimit),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_ODB_PACKED_PRIORITY => {
                Ok(Self::SetOdbPackedPriority)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_SET_ODB_LOOSE_PRIORITY => Ok(Self::SetOdbLoosePriority),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_EXTENSIONS => Ok(Self::GetExtensions),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_EXTENSIONS => Ok(Self::SetExtensions),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_OWNER_VALIDATION => Ok(Self::GetOwnerValidation),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_OWNER_VALIDATION => Ok(Self::SetOwnerValidation),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_HOMEDIR => Ok(Self::GetHomedir),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_HOMEDIR => Ok(Self::SetHomedir),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_SERVER_CONNECT_TIMEOUT => {
                Ok(Self::SetServerConnectTimeout)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_GET_SERVER_CONNECT_TIMEOUT => {
                Ok(Self::GetServerConnectTimeout)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_SET_SERVER_TIMEOUT => Ok(Self::SetServerTimeout),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_SERVER_TIMEOUT => Ok(Self::GetServerTimeout),
            ffi::git_libgit2_opt_t_GIT_OPT_SET_USER_AGENT_PRODUCT => Ok(Self::SetUserAgentProduct),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_USER_AGENT_PRODUCT => Ok(Self::GetUserAgentProduct),
            ffi::git_libgit2_opt_t_GIT_OPT_ADD_SSL_X509_CERT => Ok(Self::AddSslX509Cert),
            ffi::git_libgit2_opt_t_GIT_OPT_GET_PACK_MAX_OBJECT_SIZE => {
                Ok(Self::GetPackMaxObjectSize)
            }
            ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECT_SIZE => {
                Ok(Self::SetPackMaxObjectSize)
            }
            value => Err(InvalidGitLibgit2Option(value)),
        }
    }
}

#[cfg(test)]
mod option_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    const OPTIONS: [GitLibgit2Option; 48] = [
        GitLibgit2Option::GetMwindowSize,
        GitLibgit2Option::SetMwindowSize,
        GitLibgit2Option::GetMwindowMappedLimit,
        GitLibgit2Option::SetMwindowMappedLimit,
        GitLibgit2Option::GetSearchPath,
        GitLibgit2Option::SetSearchPath,
        GitLibgit2Option::SetCacheObjectLimit,
        GitLibgit2Option::SetCacheMaxSize,
        GitLibgit2Option::EnableCaching,
        GitLibgit2Option::GetCachedMemory,
        GitLibgit2Option::GetTemplatePath,
        GitLibgit2Option::SetTemplatePath,
        GitLibgit2Option::SetSslCertLocations,
        GitLibgit2Option::SetUserAgent,
        GitLibgit2Option::EnableStrictObjectCreation,
        GitLibgit2Option::EnableStrictSymbolicRefCreation,
        GitLibgit2Option::SetSslCiphers,
        GitLibgit2Option::GetUserAgent,
        GitLibgit2Option::EnableOfsDelta,
        GitLibgit2Option::EnableFsyncGitdir,
        GitLibgit2Option::GetWindowsSharemode,
        GitLibgit2Option::SetWindowsSharemode,
        GitLibgit2Option::EnableStrictHashVerification,
        GitLibgit2Option::SetAllocator,
        GitLibgit2Option::EnableUnsavedIndexSafety,
        GitLibgit2Option::GetPackMaxObjects,
        GitLibgit2Option::SetPackMaxObjects,
        GitLibgit2Option::DisablePackKeepFileChecks,
        GitLibgit2Option::EnableHttpExpectContinue,
        GitLibgit2Option::GetMwindowFileLimit,
        GitLibgit2Option::SetMwindowFileLimit,
        GitLibgit2Option::SetOdbPackedPriority,
        GitLibgit2Option::SetOdbLoosePriority,
        GitLibgit2Option::GetExtensions,
        GitLibgit2Option::SetExtensions,
        GitLibgit2Option::GetOwnerValidation,
        GitLibgit2Option::SetOwnerValidation,
        GitLibgit2Option::GetHomedir,
        GitLibgit2Option::SetHomedir,
        GitLibgit2Option::SetServerConnectTimeout,
        GitLibgit2Option::GetServerConnectTimeout,
        GitLibgit2Option::SetServerTimeout,
        GitLibgit2Option::GetServerTimeout,
        GitLibgit2Option::SetUserAgentProduct,
        GitLibgit2Option::GetUserAgentProduct,
        GitLibgit2Option::AddSslX509Cert,
        GitLibgit2Option::GetPackMaxObjectSize,
        GitLibgit2Option::SetPackMaxObjectSize,
    ];

    #[test]
    fn every_published_option_round_trips() {
        for option in OPTIONS {
            let raw = ffi::git_libgit2_opt_t::from(option);
            assert_eq!(GitLibgit2Option::try_from(raw), Ok(option));
        }
    }

    #[test]
    fn unknown_options_are_rejected() {
        let raw = ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECT_SIZE + 1;
        assert_eq!(GitLibgit2Option::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn option_selector_matches_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitLibgit2Option>(),
            size_of::<ffi::git_libgit2_opt_t>()
        );
        assert_eq!(
            align_of::<GitLibgit2Option>(),
            align_of::<ffi::git_libgit2_opt_t>()
        );
    }
}
