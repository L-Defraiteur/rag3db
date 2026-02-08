#pragma once

// Helpers
#if defined _WIN32 || defined __CYGWIN__
#define RAG3DB_HELPER_DLL_IMPORT __declspec(dllimport)
#define RAG3DB_HELPER_DLL_EXPORT __declspec(dllexport)
#define RAG3DB_HELPER_DLL_LOCAL
#define RAG3DB_HELPER_DEPRECATED __declspec(deprecated)
#else
#define RAG3DB_HELPER_DLL_IMPORT __attribute__((visibility("default")))
#define RAG3DB_HELPER_DLL_EXPORT __attribute__((visibility("default")))
#define RAG3DB_HELPER_DLL_LOCAL __attribute__((visibility("hidden")))
#define RAG3DB_HELPER_DEPRECATED __attribute__((__deprecated__))
#endif

#ifdef RAG3DB_STATIC_DEFINE
#define RAG3DB_API
#else
#ifndef RAG3DB_API
#ifdef RAG3DB_EXPORTS
/* We are building this library */
#define RAG3DB_API RAG3DB_HELPER_DLL_EXPORT
#else
/* We are using this library */
#define RAG3DB_API RAG3DB_HELPER_DLL_IMPORT
#endif
#endif
#endif

#ifndef RAG3DB_DEPRECATED
#define RAG3DB_DEPRECATED RAG3DB_HELPER_DEPRECATED
#endif

#ifndef RAG3DB_DEPRECATED_EXPORT
#define RAG3DB_DEPRECATED_EXPORT RAG3DB_API RAG3DB_DEPRECATED
#endif
