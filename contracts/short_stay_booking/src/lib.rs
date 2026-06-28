#![no_std]

//! # short_stay_booking
//!
//! A Soroban smart contract that models an Airbnb-like short-stay booking
//! workflow on the Stellar network. Hosts list properties with a nightly
//! rate, guests book date ranges, the host confirms, both sides record
//! check-in / check-out, and the guest leaves a 1-5 star review after
//! the stay completes. No native asset transfer happens on-chain in this
//! reference contract; the focus is on the booking state machine and
//! tamper-proof event history.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol};

/// Storage keys for the contract's persistent state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// A property listing, keyed by its monotonically-increasing id.
    Listing(u64),
    /// A booking record, keyed by its monotonically-increasing id.
    Booking(u64),
    /// The single review attached to a given booking id.
    Review(u64),
    /// Monotonic counter used to mint the next listing id.
    NextListingId,
    /// Monotonic counter used to mint the next booking id.
    NextBookingId,
}

/// A property offered for short-stay rental.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Listing {
    /// Address of the host who owns the listing.
    pub host: Address,
    /// Off-chain content hash (e.g. an IPFS CID) describing the property.
    pub property_hash: String,
    /// Price per night, denominated in the smallest unit of the chosen asset.
    pub nightly_rate: u64,
    /// Whether the listing is currently accepting new bookings.
    pub active: bool,
}

/// A guest's booking of a listing for a date range.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Booking {
    /// Address of the guest who made the booking.
    pub guest: Address,
    /// Address of the host (copied from the listing at booking time).
    pub host: Address,
    /// Identifier of the listing being booked.
    pub listing_id: u64,
    /// Stay start timestamp (e.g. UNIX days).
    pub check_in: u64,
    /// Stay end timestamp (e.g. UNIX days), exclusive.
    pub check_out: u64,
    /// Lifecycle status: `"pending"`, `"confirmed"`, `"checked_in"`,
    /// `"checked_out"`, or `"reviewed"`.
    pub status: Symbol,
    /// Pre-computed number of nights (`check_out - check_in`).
    pub total_nights: u64,
    /// Pre-computed total price (`total_nights * nightly_rate`).
    pub total_price: u64,
}

/// A post-stay review left by a guest.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Review {
    /// Address of the guest who authored the review.
    pub guest: Address,
    /// Identifier of the booking being reviewed.
    pub booking_id: u64,
    /// Star rating, constrained to `1..=5`.
    pub rating: u32,
    /// Off-chain content hash of the review comment.
    pub comment_hash: String,
    /// Ledger timestamp at which the review was recorded.
    pub timestamp: u64,
}

#[contract]
pub struct ShortStayBooking;

#[contractimpl]
impl ShortStayBooking {
    /// Register a new short-stay listing for a property owned by `host`.
    ///
    /// The `property_hash` is an off-chain content reference (for example
    /// an IPFS CID) describing the property. `nightly_rate` is the price
    /// per night expressed in the smallest unit of the chosen settlement
    /// asset, and must be strictly positive. The host's authorization is
    /// required. Returns the newly assigned `listing_id`.
    pub fn list_property(
        env: Env,
        host: Address,
        property_hash: String,
        nightly_rate: u64,
    ) -> u64 {
        host.require_auth();

        if nightly_rate == 0 {
            panic!("nightly_rate must be positive");
        }

        let listing_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextListingId)
            .unwrap_or(0u64);

        let listing = Listing {
            host: host.clone(),
            property_hash,
            nightly_rate,
            active: true,
        };
        env.storage()
            .instance()
            .set(&DataKey::Listing(listing_id), &listing);
        env.storage()
            .instance()
            .set(&DataKey::NextListingId, &(listing_id + 1));

        listing_id
    }

    /// Book a stay at `listing_id` from `check_in` to `check_out` (both
    /// timestamps such as UNIX days). The guest's authorization is
    /// required, the listing must be active, and `check_out` must be
    /// strictly greater than `check_in`. The booking is created in the
    /// `"pending"` state and must be confirmed by the host before the
    /// guest can check in. Returns the newly assigned `booking_id`.
    pub fn book(
        env: Env,
        guest: Address,
        listing_id: u64,
        check_in: u64,
        check_out: u64,
    ) -> u64 {
        guest.require_auth();

        if check_out <= check_in {
            panic!("check_out must be after check_in");
        }

        let listing: Listing = env
            .storage()
            .instance()
            .get(&DataKey::Listing(listing_id))
            .expect("listing not found");
        if !listing.active {
            panic!("listing is not active");
        }

        let total_nights = check_out - check_in;
        let total_price = total_nights
            .checked_mul(listing.nightly_rate)
            .expect("total price overflow");

        let booking_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextBookingId)
            .unwrap_or(0u64);

        let booking = Booking {
            guest: guest.clone(),
            host: listing.host.clone(),
            listing_id,
            check_in,
            check_out,
            status: Symbol::new(&env, "pending"),
            total_nights,
            total_price,
        };
        env.storage()
            .instance()
            .set(&DataKey::Booking(booking_id), &booking);
        env.storage()
            .instance()
            .set(&DataKey::NextBookingId, &(booking_id + 1));

        booking_id
    }

    /// Move a booking from `"pending"` to `"confirmed"`. Only the host
    /// who owns the listing referenced by the booking may confirm it.
    /// Once confirmed, the guest is allowed to check in during the
    /// stay window.
    pub fn confirm(env: Env, host: Address, booking_id: u64) {
        host.require_auth();

        let mut booking: Booking = env
            .storage()
            .instance()
            .get(&DataKey::Booking(booking_id))
            .expect("booking not found");
        if booking.host != host {
            panic!("only the listing host can confirm");
        }
        if booking.status != Symbol::new(&env, "pending") {
            panic!("booking is not in a pending state");
        }

        booking.status = Symbol::new(&env, "confirmed");
        env.storage()
            .instance()
            .set(&DataKey::Booking(booking_id), &booking);
    }

    /// Record guest check-in for a confirmed booking. Only the guest
    /// who created the booking may check in, and the current ledger
    /// timestamp must fall within the `[check_in, check_out)` window.
    pub fn check_in(env: Env, guest: Address, booking_id: u64) {
        guest.require_auth();

        let mut booking: Booking = env
            .storage()
            .instance()
            .get(&DataKey::Booking(booking_id))
            .expect("booking not found");
        if booking.guest != guest {
            panic!("only the booking guest can check in");
        }
        if booking.status != Symbol::new(&env, "confirmed") {
            panic!("booking is not in a confirmed state");
        }

        let now = env.ledger().timestamp();
        if now < booking.check_in || now >= booking.check_out {
            panic!("current time is outside the stay window");
        }

        booking.status = Symbol::new(&env, "checked_in");
        env.storage()
            .instance()
            .set(&DataKey::Booking(booking_id), &booking);
    }

    /// Record host-side check-out for a booking that is currently
    /// `"checked_in"`. Only the listing's host may check a guest out.
    /// After check-out the guest is allowed to leave a review.
    pub fn check_out(env: Env, host: Address, booking_id: u64) {
        host.require_auth();

        let mut booking: Booking = env
            .storage()
            .instance()
            .get(&DataKey::Booking(booking_id))
            .expect("booking not found");
        if booking.host != host {
            panic!("only the listing host can check out");
        }
        if booking.status != Symbol::new(&env, "checked_in") {
            panic!("booking is not in a checked_in state");
        }

        booking.status = Symbol::new(&env, "checked_out");
        env.storage()
            .instance()
            .set(&DataKey::Booking(booking_id), &booking);
    }

    /// Leave a review for a completed stay. The `rating` must be in
    /// `1..=5`; `comment_hash` is an off-chain content reference for
    /// the review body. Only the booking's guest may review, the stay
    /// must have been checked out, and at most one review is allowed
    /// per booking.
    pub fn leave_review(
        env: Env,
        guest: Address,
        booking_id: u64,
        rating: u32,
        comment_hash: String,
    ) {
        guest.require_auth();

        if rating < 1 || rating > 5 {
            panic!("rating must be between 1 and 5");
        }

        let mut booking: Booking = env
            .storage()
            .instance()
            .get(&DataKey::Booking(booking_id))
            .expect("booking not found");
        if booking.guest != guest {
            panic!("only the booking guest can leave a review");
        }
        if booking.status != Symbol::new(&env, "checked_out") {
            panic!("booking is not in a checked_out state");
        }
        if env
            .storage()
            .instance()
            .has(&DataKey::Review(booking_id))
        {
            panic!("a review already exists for this booking");
        }

        let review = Review {
            guest,
            booking_id,
            rating,
            comment_hash,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .instance()
            .set(&DataKey::Review(booking_id), &review);

        booking.status = Symbol::new(&env, "reviewed");
        env.storage()
            .instance()
            .set(&DataKey::Booking(booking_id), &booking);
    }
}
