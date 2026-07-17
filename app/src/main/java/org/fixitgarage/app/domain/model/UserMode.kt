package org.fixitgarage.app.domain.model

/**
 * Setup-wizard preference: which feature set to emphasize.
 * DIY users see oil-change helpers; shop users see shop receipts; both see everything.
 */
enum class UserMode {
    DIY,
    SHOP,
    BOTH;

    companion object {
        fun fromStorage(value: String?): UserMode =
            entries.firstOrNull { it.name == value } ?: BOTH
    }
}
